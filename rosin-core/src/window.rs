use crate::alloc::Alloc;
use crate::geometry::Point;
use crate::prelude::*;
use crate::{alloc::Scope, draw, layout, layout::Layout, stylesheet, tree::*};

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bumpalo::{collections::Vec as BumpVec, Bump};
use druid_shell::piet::Piet;
use druid_shell::{KeyEvent, MouseEvent};

pub struct RosinWindow<S: 'static, H: Clone + 'static> {
    resource_loader: Arc<Mutex<ResourceLoader>>,
    view: ViewCallback<S, H>,
    size: (f32, f32),
    scale: (f32, f32),
    handle: H,
    phase: Phase,
    last_frame: Instant,
    focused_node: Option<Key>,
    hover_nodes: Vec<usize>,
    prev_hover_nodes: Vec<usize>,
    prev_hover_keys: Vec<Key>,
    anim_tasks: Rc<RefCell<Vec<Box<dyn AnimCallback<S>>>>>,
    key_map: HashMap<Key, usize>,
    tree_cache: Option<Scope<BumpVec<'static, ArrayNode<S, H>>>>,
    layout_cache: Option<Scope<BumpVec<'static, Layout>>>,
    alloc: Rc<Alloc>,
    temp: Bump,
}

impl<S, H: Default + Clone> RosinWindow<S, H> {
    pub fn new(resource_loader: Arc<Mutex<ResourceLoader>>, view: ViewCallback<S, H>, size: (f32, f32), handle: H) -> Self {
        Self {
            resource_loader,
            view,
            size,
            scale: (1.0, 1.0),
            handle,
            phase: Phase::Build,
            last_frame: Instant::now(),
            focused_node: None,
            hover_nodes: Vec::new(),
            prev_hover_nodes: Vec::new(),
            prev_hover_keys: Vec::new(),
            anim_tasks: Rc::new(RefCell::new(Vec::new())),
            key_map: HashMap::new(),
            tree_cache: None,
            layout_cache: None,
            alloc: Rc::new(Alloc::default()),
            temp: Bump::new(),
        }
    }

    pub fn set_handle(&mut self, handle: H) {
        self.handle = handle;
    }

    pub fn reset_cache(&mut self) {
        self.layout_cache = None;
        self.tree_cache = None;
        self.alloc.reset().expect("[Rosin] Failed to reset cache");

        self.prev_hover_nodes.clear();
        self.key_map.clear();
    }

    pub fn get_alloc(&self) -> Rc<Alloc> {
        self.alloc.clone()
    }

    pub fn update_phase(&mut self, new_phase: Phase) {
        self.phase.update(new_phase);
    }

    pub fn size(&mut self, new_size: (f32, f32)) {
        self.size.0 = new_size.0;
        self.size.1 = new_size.1;
        self.update_phase(Phase::Layout);
    }

    pub fn scale(&mut self, new_scale: (f32, f32)) {
        self.scale = new_scale;
    }

    pub fn set_view(&mut self, new_view: ViewCallback<S, H>) {
        self.view = new_view;
        self.phase = Phase::Build;
    }

    pub fn is_idle(&self) -> bool {
        self.phase == Phase::Idle
    }

    pub fn has_anim_tasks(&self) -> bool {
        self.anim_tasks.borrow().len() > 0
    }

    pub fn add_anim_task(&mut self, callback: impl Fn(&mut S, Duration) -> (Phase, ShouldStop) + 'static) {
        self.anim_tasks.borrow_mut().push(Box::new(callback));
    }

    pub fn got_focus(&mut self, state: &mut S) {
        self.root_event(state, On::WindowFocus);
    }

    pub fn lost_focus(&mut self, state: &mut S) {
        self.root_event(state, On::WindowFocus);
    }

    pub fn close(&mut self, state: &mut S) {
        self.root_event(state, On::WindowClose);
    }

    fn root_event(&mut self, state: &mut S, event_type: On) {
        if let Some(tree) = &mut self.tree_cache {
            let tree = tree.borrow_mut();

            let default_style = Style::default();
            let default_layout = Layout::default();

            let mut ctx = EventCtx {
                event_info: EventInfo::None,
                window_handle: self.handle.clone(),
                resource_loader: self.resource_loader.clone(),
                focus: self.focused_node,
                style: default_style,
                layout: default_layout,
                change: false,
                anim_tasks: self.anim_tasks.clone(),
            };

            let mut phase = Self::dispatch_event(event_type, state, &mut ctx, tree, 0);
            phase.update(self.handle_ctx(state, ctx));
            self.update_phase(phase);
        }
    }

    pub fn mouse_leave(&mut self, state: &mut S) {
        if let Some(tree) = &mut self.tree_cache {
            let tree = tree.borrow_mut();

            let default_style = Style::default();
            let default_layout = Layout::default();

            let mut ctx = EventCtx {
                event_info: EventInfo::None,
                window_handle: self.handle.clone(),
                resource_loader: self.resource_loader.clone(),
                focus: self.focused_node,
                style: default_style,
                layout: default_layout,
                change: false,
                anim_tasks: self.anim_tasks.clone(),
            };

            if self.prev_hover_nodes.is_empty() {
                for key in &self.prev_hover_keys {
                    if let Some(&id) = self.key_map.get(key) {
                        self.prev_hover_nodes.push(id);
                    }
                }
            }

            // Dispatch MouseLeave event to all previously hovered nodes
            let mut phase = Phase::Idle;
            for &id in &self.prev_hover_nodes {
                phase.update(Self::dispatch_event(On::MouseLeave, state, &mut ctx, tree, id));
            }
            phase.update(self.handle_ctx(state, ctx));
            self.update_phase(phase);

            // The mouse has left the window, so it's not hovering over anything this frame
            self.prev_hover_nodes.clear();
            self.prev_hover_keys.clear();
        }
    }

    pub fn mouse_wheel(&mut self, state: &mut S, event: &MouseEvent) {
        self.mouse_event(state, event, On::MouseWheel)
    }

    pub fn mouse_move(&mut self, state: &mut S, event: &MouseEvent) {
        self.mouse_event(state, event, On::MouseMove)
    }

    pub fn mouse_down(&mut self, state: &mut S, event: &MouseEvent) {
        self.mouse_event(state, event, On::MouseDown)
    }

    pub fn mouse_up(&mut self, state: &mut S, event: &MouseEvent) {
        self.mouse_event(state, event, On::MouseUp)
    }

    fn mouse_event(&mut self, state: &mut S, event: &MouseEvent, event_type: On) {
        if let (Some(tree), Some(layout)) = (&mut self.tree_cache, &self.layout_cache) {
            let tree = tree.borrow_mut();
            let layout = layout.borrow();
            self.temp.reset();

            let default_style = Style::default();
            let default_layout = Layout::default();

            let mut ctx = EventCtx {
                event_info: EventInfo::Mouse(event.clone()),
                window_handle: self.handle.clone(),
                resource_loader: self.resource_loader.clone(),
                focus: self.focused_node,
                style: default_style,
                layout: default_layout,
                change: false,
                anim_tasks: self.anim_tasks.clone(),
            };

            let position = Point {
                x: event.pos.x as f32,
                y: event.pos.y as f32,
            };

            // Get ids of nodes the mouse is over
            self.hover_nodes.clear();
            layout::hit_test(layout, position, &mut self.hover_nodes);

            // If there are no hovered ids from the previous frame, the tree might have been rebuilt
            // So, use keys to get the ids of previously hovered nodes
            if self.prev_hover_nodes.is_empty() {
                for key in &self.prev_hover_keys {
                    if let Some(&id) = self.key_map.get(key) {
                        self.prev_hover_nodes.push(id);
                    }
                }
                self.prev_hover_nodes.sort_unstable();
            }

            let mut mouse_enter_nodes: BumpVec<usize> = BumpVec::new_in(&self.temp);
            let mut mouse_leave_nodes: BumpVec<usize> = BumpVec::new_in(&self.temp);

            let mut curr: usize = 0;
            let mut prev: usize = 0;

            // Compare hovered nodes with previous frame
            // NOTE: Assumes the vecs are sorted ascending
            while curr < self.hover_nodes.len() && prev < self.prev_hover_nodes.len() {
                match self.hover_nodes[curr].cmp(&self.prev_hover_nodes[prev]) {
                    Ordering::Less => {
                        mouse_enter_nodes.push(self.hover_nodes[curr]);
                        curr += 1;
                    }
                    Ordering::Greater => {
                        mouse_leave_nodes.push(self.prev_hover_nodes[prev]);
                        prev += 1;
                    }
                    Ordering::Equal => {
                        curr += 1;
                        prev += 1;
                    }
                }
            }
            while curr < self.hover_nodes.len() {
                mouse_enter_nodes.push(self.hover_nodes[curr]);
                curr += 1;
            }
            while prev < self.prev_hover_nodes.len() {
                mouse_leave_nodes.push(self.prev_hover_nodes[prev]);
                prev += 1;
            }

            // Dispatch events
            let mut phase = Phase::Idle;

            for id in mouse_leave_nodes {
                ctx.style = tree[id].style.clone();
                ctx.layout = layout[id];
                phase.update(Self::dispatch_event(On::MouseLeave, state, &mut ctx, tree, id));
            }

            for id in mouse_enter_nodes {
                ctx.style = tree[id].style.clone();
                ctx.layout = layout[id];
                phase.update(Self::dispatch_event(On::MouseEnter, state, &mut ctx, tree, id));
            }

            for &id in &self.hover_nodes {
                ctx.style = tree[id].style.clone();
                ctx.layout = layout[id];
                phase.update(Self::dispatch_event(event_type, state, &mut ctx, tree, id));
            }

            // Store the keys from hovered nodes in case the tree gets rebuilt
            std::mem::swap(&mut self.hover_nodes, &mut self.prev_hover_nodes);
            self.prev_hover_keys.clear();
            for &id in &self.prev_hover_nodes {
                if let Some(key) = tree[id].key {
                    self.prev_hover_keys.push(key);
                }
            }

            phase.update(self.handle_ctx(state, ctx));
            self.update_phase(phase);
        }
    }

    pub fn key_down(&mut self, state: &mut S, event: KeyEvent) -> bool {
        self.key_event(state, event, On::KeyDown)
    }

    pub fn key_up(&mut self, state: &mut S, event: KeyEvent) {
        self.key_event(state, event, On::KeyUp);
    }

    fn key_event(&mut self, state: &mut S, event: KeyEvent, event_type: On) -> bool {
        if let Some(tree) = &mut self.tree_cache {
            let tree = tree.borrow_mut();

            // Find the id of the focused node, or route event to root node
            let id = if let Some(key) = &self.focused_node {
                if let Some(&id) = self.key_map.get(key) {
                    id
                } else {
                    0
                }
            } else {
                0
            };

            let default_style = Style::default();
            let default_layout = Layout::default();

            if tree[id].has_callback(event_type) {
                let mut ctx = EventCtx {
                    event_info: EventInfo::Key(event),
                    window_handle: self.handle.clone(),
                    resource_loader: self.resource_loader.clone(),
                    focus: self.focused_node,
                    style: default_style,
                    layout: default_layout,
                    change: false,
                    anim_tasks: self.anim_tasks.clone(),
                };

                let mut phase = Self::dispatch_event(event_type, state, &mut ctx, tree, id);
                phase.update(self.handle_ctx(state, ctx));
                self.update_phase(phase);
                return true;
            }
        }
        false
    }

    fn dispatch_event(event_type: On, state: &mut S, ctx: &mut EventCtx<S, H>, tree: &mut [ArrayNode<S, H>], id: usize) -> Phase {
        ctx.change = false;
        let mut phase = tree[id].run_callbacks(event_type, state, ctx);

        // If requested, dispatch a change event
        if ctx.change {
            let default_style = Style::default();
            let default_layout = Layout::default();

            let mut change_ctx: EventCtx<S, H> = EventCtx {
                event_info: EventInfo::None,
                window_handle: ctx.window_handle.clone(),
                resource_loader: ctx.resource_loader.clone(),
                focus: ctx.focus,
                style: default_style,
                layout: default_layout,
                change: false,
                anim_tasks: ctx.anim_tasks.clone(),
            };

            if event_type != On::Change && tree[id].has_callback(On::Change) {
                phase.update(Self::dispatch_event(On::Change, state, &mut change_ctx, tree, id));
            } else {
                // Search up tree for change event handler
                let mut curr = tree[id].parent;
                while curr != 0 {
                    if tree[curr].has_callback(On::Change) {
                        phase.update(Self::dispatch_event(On::Change, state, &mut change_ctx, tree, curr));
                        ctx.focus = change_ctx.focus;
                        return phase;
                    }
                    curr = tree[curr].parent;
                }

                // Check root node
                if id != 0 && tree[0].has_callback(On::Change) {
                    phase.update(Self::dispatch_event(On::Change, state, &mut change_ctx, tree, 0));
                }
            }

            ctx.focus = change_ctx.focus;
        }

        phase
    }

    fn handle_ctx(&mut self, state: &mut S, ctx: EventCtx<S, H>) -> Phase {
        let mut phase = Phase::Idle;

        if let Some(tree) = &mut self.tree_cache {
            let tree = tree.borrow_mut();

            let default_style = Style::default();
            let default_layout = Layout::default();

            let mut focus_ctx: EventCtx<S, H> = EventCtx {
                event_info: EventInfo::None,
                window_handle: ctx.window_handle.clone(),
                resource_loader: ctx.resource_loader.clone(),
                focus: ctx.focus,
                style: default_style,
                layout: default_layout,
                change: false,
                anim_tasks: self.anim_tasks.clone(),
            };

            // Dispatch focus and blur events
            match (self.focused_node, ctx.focus) {
                (Some(blur_key), Some(focus_key)) => {
                    if blur_key != focus_key {
                        if let Some(&id) = self.key_map.get(&blur_key) {
                            phase.update(Self::dispatch_event(On::Blur, state, &mut focus_ctx, tree, id));
                        }
                        if let Some(&id) = self.key_map.get(&focus_key) {
                            phase.update(Self::dispatch_event(On::Focus, state, &mut focus_ctx, tree, id));
                        }
                    }
                }
                (Some(blur_key), None) => {
                    if let Some(&id) = self.key_map.get(&blur_key) {
                        phase.update(Self::dispatch_event(On::Blur, state, &mut focus_ctx, tree, id));
                    }
                }
                (None, Some(focus_key)) => {
                    if let Some(&id) = self.key_map.get(&focus_key) {
                        phase.update(Self::dispatch_event(On::Focus, state, &mut focus_ctx, tree, id));
                    }
                }
                (None, None) => {}
            }
        }

        self.focused_node = ctx.focus;

        phase
    }

    pub fn animation_frame(&mut self, state: &mut S) {
        // Get time since last frame
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame);
        self.last_frame = now;

        // Run Animation Tasks
        let mut anim_phase = Phase::Idle;
        self.anim_tasks.borrow_mut().retain(|task| {
            let (phase, stop) = task(state, dt);
            anim_phase.update(phase);
            stop == ShouldStop::No
        });
        self.update_phase(anim_phase);
    }

    pub fn draw(&mut self, state: &S, piet: Option<&mut Piet<'_>>) -> Result<(), Box<dyn Error>> {
        // Set up allocators
        Alloc::set_thread_local_alloc(Some(self.alloc.clone()));
        let alloc = self.alloc.clone();
        self.temp.reset();

        // ---------- Build Phase ----------
        if self.phase == Phase::Build || self.tree_cache.is_none() {
            self.reset_cache();

            // Reset counter so we can track how many nodes are allocated
            alloc.reset_counter();

            // SAFETY: This is safe because we panic if client code breaks scope()'s contract
            let mut tree = unsafe {
                // Run the view function
                alloc.scope(|| (self.view)(state).finish(&mut self.key_map).unwrap())
            };

            // Panic if the view function didn't return the number of nodes we expected
            assert!(alloc.get_counter() == tree.borrow().len(), "[Rosin] Nodes missing");

            stylesheet::apply_styles(&self.temp, tree.borrow_mut());
            self.tree_cache = Some(tree);
        }

        let tree: &mut BumpVec<ArrayNode<S, H>> = self.tree_cache.as_mut().unwrap().borrow_mut();

        // Stash default styles, apply hover/focus styles, and run style callbacks
        let mut default_styles: BumpVec<(usize, Style)> = BumpVec::new_in(&self.temp);
        if self.phase != Phase::Idle {
            for (id, node) in tree.iter_mut().enumerate() {
                // TODO - apply hover/focus styles using prev_hover_nodes/keys
                //      - set phase to layout if needed

                if let Some(style_callback) = &mut node.style_callback {
                    default_styles.push((id, node.style.clone()));
                    style_callback(state, &mut node.style);
                }
            }
        }

        // ---------- Layout Phase ----------
        if self.phase >= Phase::Layout || self.layout_cache.is_none() {
            if self.layout_cache.is_none() {
                let new_layout = unsafe {
                    // SAFETY: This is safe because we meet scope()'s requirements
                    alloc.scope(|| alloc.vec_capacity(tree.len()))
                };

                self.layout_cache = Some(new_layout);
            }

            let layout = self.layout_cache.as_mut().unwrap().borrow_mut();

            layout.clear();
            for _ in 0..tree.len() {
                layout.push(Layout::default());
            }

            layout::layout(&self.temp, tree, self.size.into(), layout);

            for (id, node) in tree.iter_mut().enumerate() {
                if let Some(layout_callback) = &mut node.layout_callback {
                    layout_callback(state, layout[id].size);
                }
            }
        }

        let layout: &BumpVec<Layout> = self.layout_cache.as_ref().unwrap().borrow();

        // ---------- Draw Phase ----------
        // TODO - If phase == Idle, re-issue commands from last frame
        if let Some(piet) = piet {
            draw::draw(state, tree, layout, piet);
        }

        // ---------- Cleanup ----------
        Alloc::set_thread_local_alloc(None);
        self.phase = Phase::Idle;

        // Restore default styles
        for (id, style) in default_styles {
            tree[id].style = style;
        }

        Ok(())
    }
}
