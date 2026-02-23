//! The main entry point to the rosin-core library.

use std::{
    cell::Cell,
    cmp::Ordering,
    collections::VecDeque,
    fmt,
    time::{Duration, Instant},
};

use accesskit::{Action, ActionRequest, Node as AxNode, NodeId as AxNodeId, Rect as AxRect, Role, Tree, TreeId, TreeUpdate};
use bumpalo::{Bump, collections::Vec as BumpVec};
use keyboard_types::KeyboardEvent;
use kurbo::{Point, Size, Vec2};
use log::error;
use qfilter::Filter;
use vello::Scene;

use crate::{
    prelude::*,
    reactive::Registry,
    {css, draw, layout},
};

const MAX_EVENT_DEPTH: u32 = 50;

/// Returned when building the AccessKit tree fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessKitUpdateError {
    TreeNotReady,
}

impl fmt::Display for AccessKitUpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TreeNotReady => write!(f, "Tree not ready"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EventId(pub u64);

#[derive(Debug)]
struct QueuedEvent {
    idx: usize,
    event_id: EventId,
    event_type: On,
    event_info: EventInfo,
    depth: u32,
}

impl QueuedEvent {
    fn new(idx: usize, event_id: EventId, event_type: On, event_info: EventInfo) -> Self {
        Self {
            idx,
            event_id,
            event_type,
            event_info,
            depth: 0,
        }
    }
}

#[derive(Debug, Copy, PartialEq, Eq, PartialOrd, Ord, Clone)]
enum Phase {
    Idle = 0,
    Draw = 1,
    Layout = 2,
    Build = 4,
}

impl Phase {
    pub fn update(&mut self, other: Phase) {
        *self = (*self).max(other);
    }
}

/// The main entry point to the rosin-core library.
///
/// Use this if you're building your own platform integration.
pub struct Viewport<S: Sync + 'static, H: Clone + 'static> {
    /// The callback that builds the ui tree
    view_callback: fn(&S, &mut Ui<S, H>),

    /// The size of the window
    size: Size,

    /// The scale of the window
    scale: Vec2,

    /// The phase of a frame that should be skipped to on next frame
    phase: Phase,

    /// The currently active node
    active_node: Option<NodeId>,

    /// The currently focused node
    focused_node: Option<NodeId>,

    /// The node that is capturing pointer events
    capture: Option<NodeId>,

    /// The last received pointer event
    last_pointer_event: Option<PointerEvent>,

    /// The distance the pointer moved to arrive at `last_pointer_event.pos`
    pointer_delta: Option<Vec2>,

    /// The nodes that are below the pointer.
    curr_hot_indexes: Vec<usize>,

    /// The nodes that were below the pointer last frame. These are indexes into curr_tree.nodes
    prev_hot_indexes: Vec<usize>,

    /// The UI tree for this frame
    pub(crate) curr_tree: Ui<S, H>,

    /// The UI tree from last frame
    prev_tree: Ui<S, H>,

    /// Events waiting to be processed
    event_queue: VecDeque<QueuedEvent>,

    /// The rendered scene
    scene_cache: Scene,

    /// A temporary allocator
    temp: Bump,

    /// A shared reference to the global translation map
    translation_map: TranslationMap,

    /// Record of how long the different phases of the last frame took.
    perf_info: PerfInfo,

    /// Used to check if anything was changed since last frame
    last_write_count: u64,

    /// Variables that the Build phase depends on
    build_deps: DependencyMap,

    /// Variables that the Style phase depends on
    style_deps: DependencyMap,

    /// Variables that the Layout phase depends on
    layout_deps: DependencyMap,

    /// Variables that the Draw phase depends on
    draw_deps: DependencyMap,

    /// The ID that will be assigned to the next event
    next_event_id: Cell<EventId>,

    /// Used when selector matching
    ancestor_classes: Filter,
}

fn sorted_iter_diff<'a, I, T>(mut curr_iter: I, mut prev_iter: I, new_items: &mut BumpVec<T>, old_items: &mut BumpVec<T>)
where
    I: Iterator<Item = &'a T> + 'a,
    T: Ord + PartialOrd + Copy + 'a,
{
    let mut curr = curr_iter.next();
    let mut prev = prev_iter.next();

    while let (Some(c), Some(p)) = (curr, prev) {
        match c.cmp(p) {
            Ordering::Less => {
                new_items.push(*c);
                curr = curr_iter.next();
            }
            Ordering::Greater => {
                old_items.push(*p);
                prev = prev_iter.next();
            }
            Ordering::Equal => {
                curr = curr_iter.next();
                prev = prev_iter.next();
            }
        }
    }

    while let Some(item) = curr {
        new_items.push(*item);
        curr = curr_iter.next();
    }

    while let Some(item) = prev {
        old_items.push(*item);
        prev = prev_iter.next();
    }
}

impl<S: Sync, H: Clone> Viewport<S, H> {
    pub fn new(view_callback: fn(&S, &mut Ui<S, H>), size: Size, scale: Vec2, translation_map: TranslationMap) -> Self {
        Self {
            view_callback,
            size,
            scale,
            phase: Phase::Build,
            active_node: None,
            focused_node: None,
            capture: None,
            last_pointer_event: None,
            pointer_delta: None,
            curr_hot_indexes: Vec::new(),
            prev_hot_indexes: Vec::new(),
            curr_tree: Ui::new(),
            prev_tree: Ui::new(),
            event_queue: VecDeque::new(),
            scene_cache: Scene::new(),
            temp: Bump::new(),
            translation_map,
            perf_info: PerfInfo::default(),
            last_write_count: Registry::global().write_count(),
            build_deps: DependencyMap::default(),
            style_deps: DependencyMap::default(),
            layout_deps: DependencyMap::default(),
            draw_deps: DependencyMap::default(),
            next_event_id: Cell::new(EventId(0)),
            ancestor_classes: Filter::new(250, 0.01).unwrap(), // Unwrap ok: values are hardcoded and reasonable
        }
    }

    fn next_event_id(&self) -> EventId {
        let result = self.next_event_id.get();
        self.next_event_id.update(|id| EventId(id.0 + 1));
        result
    }

    fn update_hot_indexes_at(&mut self, point: Point) {
        self.curr_hot_indexes.clear();
        layout::hit_test(&self.temp, &self.curr_tree, point, &mut self.curr_hot_indexes);
    }

    // Returns true if any nodes were marked dirty
    fn mark_hot_dirty(&mut self) -> bool {
        let mut pointer_enter_nodes: BumpVec<usize> = BumpVec::with_capacity_in(self.curr_hot_indexes.len(), &self.temp);
        let mut pointer_leave_nodes: BumpVec<usize> = BumpVec::with_capacity_in(self.prev_hot_indexes.len(), &self.temp);

        sorted_iter_diff(self.curr_hot_indexes.iter(), self.prev_hot_indexes.iter(), &mut pointer_enter_nodes, &mut pointer_leave_nodes);

        let mut dirtied = false;
        for &idx in pointer_enter_nodes.iter().chain(pointer_leave_nodes.iter()) {
            let style_flags = self.curr_tree.style_flags.get(idx).copied().unwrap_or(0);

            if (style_flags & css::HOVER_DIRTY) == 0 {
                continue;
            }

            self.curr_tree.dirty_roots.push(idx);
            dirtied = true;
        }

        dirtied
    }

    pub fn get_focused_node(&self) -> Option<NodeId> {
        self.focused_node
    }

    pub fn get_active_node(&self) -> Option<NodeId> {
        self.active_node
    }

    pub fn get_size(&self) -> Size {
        self.size
    }

    pub fn get_scale(&self) -> Vec2 {
        self.scale
    }

    pub fn get_translation_map(&self) -> TranslationMap {
        self.translation_map.clone()
    }

    pub fn require_build(&mut self) {
        self.phase = Phase::Build;
    }

    pub fn require_layout(&mut self) {
        self.phase.update(Phase::Layout);
    }

    pub fn require_draw(&mut self) {
        self.phase.update(Phase::Draw);
    }

    pub fn set_size(&mut self, new_size: Size) {
        self.pointer_delta = None;
        self.last_pointer_event = None;
        self.size = new_size;
        self.phase.update(Phase::Layout);
    }

    pub fn set_scale(&mut self, scale: Vec2) {
        self.pointer_delta = None;
        self.last_pointer_event = None;
        self.scale = scale;
        self.phase.update(Phase::Layout);
    }

    pub fn set_view_callback(&mut self, view_callback: fn(&S, &mut Ui<S, H>)) {
        self.view_callback = view_callback;
        self.phase = Phase::Build;
    }

    pub fn report_paint_time(&mut self, time: Duration) {
        self.perf_info.paint_time = time;
    }

    pub fn get_perf_info(&mut self) -> PerfInfo {
        self.perf_info
    }

    pub fn frame_number(&self) -> u64 {
        self.perf_info.frame_number
    }

    pub fn is_idle(&self) -> bool {
        self.phase == Phase::Idle && self.event_queue.is_empty()
    }

    pub fn has_anim_nodes(&self) -> bool {
        !self.curr_tree.on_anim_nodes.is_empty()
    }

    /// Reloads the translation map if any of the files have been modified since they were last loaded.
    ///
    /// Returns `true` if any of the files were successfully reloaded.
    ///
    /// ## Errors
    ///
    /// The method will return a boolean irrelevant of errors, but in case of errors,
    /// the `Err` variant will also contain a Vec of any io errors encountered.
    pub fn reload_translation_map(&mut self) -> Result<bool, (bool, Vec<std::io::Error>)> {
        let reloaded = self.translation_map.reload()?;
        if reloaded {
            self.phase.update(Phase::Layout);
        }
        Ok(reloaded)
    }

    /// Reloads any stylesheets in the tree that have been modified since they were last loaded.
    ///
    /// Returns `true` if any of the files were successfully reloaded.
    ///
    /// ## Errors
    ///
    /// The method will return a boolean irrelevant of errors, but in case of errors,
    /// the `Err` variant will also contain a Vec of any io errors encountered.
    pub fn reload_stylesheets(&mut self) -> Result<bool, (bool, Vec<std::io::Error>)> {
        let mut reloaded = false;
        let mut errors = Vec::new();
        for node in &mut self.curr_tree.nodes {
            if let Some(style_sheet) = &mut node.style_sheet {
                match style_sheet.reload() {
                    Ok(did_load) => reloaded |= did_load,
                    Err(error) => errors.push(error),
                }
            }
        }
        if reloaded {
            self.phase = Phase::Build;
        }
        if !errors.is_empty() { Err((reloaded, errors)) } else { Ok(reloaded) }
    }

    pub fn build_accesskit_tree(&mut self, state: &S) -> Result<accesskit::TreeUpdate, AccessKitUpdateError> {
        let tree = &self.curr_tree;
        if tree.nodes.is_empty() || tree.layout_cache.len() != tree.nodes.len() {
            return Err(AccessKitUpdateError::TreeNotReady);
        }

        let n = tree.nodes.len();
        let root_ax: AxNodeId = tree.nodes[0].nid.map(Into::into).unwrap_or_else(|| AxNodeId::from(0));

        let mut nodes_out: Vec<(AxNodeId, AxNode)> = Vec::with_capacity(n);

        self.temp.reset();
        let mut emitted_id: BumpVec<Option<AxNodeId>> = bumpalo::vec![in &self.temp; None; n];
        let mut promoted_start: BumpVec<usize> = bumpalo::vec![in &self.temp; 0; n];
        let mut promoted_len: BumpVec<usize> = bumpalo::vec![in &self.temp; 0; n];
        let mut children: BumpVec<usize> = BumpVec::with_capacity_in(tree.max_children, &self.temp);

        let mut stack: BumpVec<(usize, bool)> = BumpVec::with_capacity_in(n * 2, &self.temp);
        stack.push((0, false));

        let mut promoted_pool: BumpVec<AxNodeId> = BumpVec::with_capacity_in(tree.max_children.max(8), &self.temp);

        while let Some((idx, expanded)) = stack.pop() {
            if !expanded {
                stack.push((idx, true));

                tree.child_indexes(idx, &mut children);
                for &c in children.iter() {
                    stack.push((c, false));
                }
                continue;
            }

            let is_root = idx == 0;
            let node = &tree.nodes[idx];

            let focusable = node.has_callback(On::Focus);
            let has_callback = node.accessibility_callback.is_some();

            // Resolve label when there's no callback.
            // If there is a callback, it's responsible for setting the label.
            let mut resolved_label = None;
            if !has_callback && let Some(text) = node.text.as_ref() {
                resolved_label = text.resolve(&self.translation_map).filter(|s| !s.is_empty());
            }

            let has_text = resolved_label.is_some();
            let meaningful = is_root || focusable || has_callback || has_text;

            // If a meaningful non-root node is missing id, build a best-effort tree and log an error.
            let ax_id: Option<AxNodeId> = if is_root {
                Some(root_ax)
            } else if meaningful {
                match node.nid {
                    Some(nid) => Some(nid.into()),
                    None => {
                        error!("AccessKit: meaningful node at idx={} missing id, skipping node in accessibility tree.", idx);
                        None
                    }
                }
            } else {
                None
            };

            // Compute flattened children for this node
            let pool_start = promoted_pool.len();

            tree.child_indexes(idx, &mut children);
            for &child_idx in children.iter() {
                if let Some(child_ax) = emitted_id[child_idx] {
                    promoted_pool.push(child_ax);
                } else {
                    let s = promoted_start[child_idx];
                    let l = promoted_len[child_idx];
                    for i in 0..l {
                        promoted_pool.push(promoted_pool[s + i]);
                    }
                }
            }

            let pool_end = promoted_pool.len();
            promoted_start[idx] = pool_start;
            promoted_len[idx] = pool_end - pool_start;

            // If not emitted, we're done
            let Some(ax_id) = ax_id else { continue };
            emitted_id[idx] = Some(ax_id);

            // Build the AccessKit node
            let rect = tree.layout_cache[idx].rect();
            let bounds = AxRect {
                x0: rect.x0 * self.scale.x,
                y0: rect.y0 * self.scale.y,
                x1: rect.x1 * self.scale.x,
                y1: rect.y1 * self.scale.y,
            };

            let default_role = if is_root { Role::Window } else { Role::GenericContainer };
            let mut ax_node = AxNode::new(default_role);
            ax_node.set_bounds(bounds);

            if !node.enabled.get_or(true) {
                ax_node.set_disabled();
            } else if focusable {
                ax_node.add_action(Action::Focus);
            }

            // Children (flattened meaningful descendants)
            let child_count = promoted_len[idx];
            if child_count != 0 {
                let s = promoted_start[idx];
                let mut ax_children = Vec::with_capacity(child_count);
                for i in 0..child_count {
                    ax_children.push(promoted_pool[s + i]);
                }
                ax_node.set_children(ax_children);
            }

            if let Some(callback) = node.accessibility_callback.as_ref() {
                if let Some(id) = node.nid {
                    let mut ctx = crate::events::AccessibilityCtx {
                        id,
                        text: node.text.as_ref(),
                        translation_map: self.translation_map.clone(),
                        node: &mut ax_node,
                    };
                    (callback)(state, &mut ctx);
                } else if is_root {
                    error!("AccessKit: root node has an accessibility callback but no id. Skipping callback.");
                }
            } else if let Some(resolved) = resolved_label {
                ax_node.set_label(resolved);
                ax_node.set_role(Role::Label);
            }

            nodes_out.push((ax_id, ax_node));
        }

        let focus = self
            .focused_node
            .filter(|nid| self.curr_tree.nid_map.contains_key(nid))
            .map(Into::into)
            .unwrap_or(root_ax);

        Ok(TreeUpdate {
            tree_id: TreeId::ROOT,
            tree: Some(Tree::new(root_ax)),
            nodes: nodes_out,
            focus,
        })
    }

    /// Queue a synthetic primary button click in the center of a node.
    ///
    /// Useful for testing and automation.
    pub fn synthesize_click(&mut self, node: NodeId) {
        let Some(&idx) = self.curr_tree.nid_map.get(&node) else {
            return;
        };

        if idx >= self.curr_tree.layout_cache.len() {
            return;
        }

        let rect = self.curr_tree.layout_cache[idx].rect();
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
        }

        let pos = Point::new((rect.x0 + rect.x1) * 0.5, (rect.y0 + rect.y1) * 0.5);

        self.queue_pointer_move_event(&PointerEvent::synthetic_move(pos));
        self.queue_pointer_down_event(&PointerEvent::synthetic_primary_down(pos, 1));
        self.queue_pointer_up_event(&PointerEvent::synthetic_primary_up(pos));
    }

    pub fn queue_accessibility_action_event(&mut self, request: ActionRequest) {
        let idx = match request.target_node.0 {
            0 => Some(0),
            t => std::num::NonZeroU64::new(t)
                .map(NodeId)
                .and_then(|nid| self.curr_tree.nid_map.get(&nid).copied()),
        };

        let Some(idx) = idx else {
            error!("{:?} event dropped: AccessKit target ({:?}) not found in tree.", On::AccessibilityAction, request.target_node);
            return;
        };

        self.event_queue
            .push_back(QueuedEvent::new(idx, self.next_event_id(), On::AccessibilityAction, EventInfo::AccessibilityAction(request)));
    }

    /// Queues an `On::Change` event for `node`.
    ///
    /// If `node` does not have an `On::Change` callback, this will walk up the
    /// parent chain and queue the event on the first ancestor that does.
    /// If no ancestor handles `On::Change`, no event is queued.
    ///
    /// This makes externally queued change events behave like [`EventCtx::emit_change`],
    /// where changes are handled by the nearest change handler in the tree rather
    /// than requiring the leaf node itself to have an `On::Change` callback.
    pub fn queue_change_event(&mut self, node: NodeId) {
        let Some(&start_idx) = self.curr_tree.nid_map.get(&node) else {
            error!("{:?} event dropped: NodeId ({:?}) not found in tree.", On::Change, node.0);
            return;
        };

        // Resolve the actual target idx: self if it handles Change, else first ancestor that does.
        let mut target_idx = start_idx;

        if !self.curr_tree.nodes[target_idx].has_callback(On::Change) {
            let mut curr = self.curr_tree.nodes[target_idx].parent;
            let mut found = None;

            while curr != usize::MAX {
                if self.curr_tree.nodes[curr].has_callback(On::Change) {
                    found = Some(curr);
                    break;
                }
                curr = self.curr_tree.nodes[curr].parent;
            }

            let Some(idx) = found else {
                // No Change handler anywhere up the tree.
                return;
            };

            target_idx = idx;
        }

        let event_id = self.next_event_id();
        self.event_queue.push_back(QueuedEvent::new(target_idx, event_id, On::Change, EventInfo::None));
    }

    pub fn queue_command_event(&mut self, node: Option<NodeId>, command: CommandId) {
        let Some(node) = node else {
            // Send to root node by default.
            let event_id = self.next_event_id();
            self.event_queue
                .push_back(QueuedEvent::new(0, event_id, On::Command, EventInfo::Command(command)));
            return;
        };

        let Some(&idx) = self.curr_tree.nid_map.get(&node) else {
            error!("{:?} event dropped: NodeId ({:?}) not found in tree.", command, node.0);
            return;
        };
        let event_id = self.next_event_id();
        self.event_queue
            .push_back(QueuedEvent::new(idx, event_id, On::Command, EventInfo::Command(command)));
    }

    pub fn queue_timer_event(&mut self, node: NodeId) {
        let Some(&idx) = self.curr_tree.nid_map.get(&node) else {
            error!("{:?} event dropped: NodeId ({:?}) not found in tree.", On::Timer, node);
            return;
        };
        let event_id = self.next_event_id();
        self.event_queue.push_back(QueuedEvent::new(idx, event_id, On::Timer, EventInfo::None));
    }

    pub fn queue_file_dialog_event(&mut self, node: NodeId, response: FileDialogResponse) {
        let Some(&idx) = self.curr_tree.nid_map.get(&node) else {
            error!("{:?} event dropped: NodeId ({:?}) not found in tree.", On::FileDialog, node.0);
            return;
        };
        let event_id = self.next_event_id();
        self.event_queue
            .push_back(QueuedEvent::new(idx, event_id, On::FileDialog, EventInfo::File(response)));
    }

    pub fn queue_got_focus_event(&mut self) {
        let event_id = self.next_event_id();
        self.event_queue.push_back(QueuedEvent::new(0, event_id, On::WindowFocus, EventInfo::None));
    }

    pub fn queue_lost_focus_event(&mut self) {
        let event_id = self.next_event_id();
        self.event_queue.push_back(QueuedEvent::new(0, event_id, On::WindowBlur, EventInfo::None));
    }

    pub fn queue_close_event(&mut self) {
        let event_id = self.next_event_id();
        self.event_queue.push_back(QueuedEvent::new(0, event_id, On::WindowClose, EventInfo::None));
    }

    pub fn queue_animation_events(&mut self, dt: Duration) {
        let event_id = self.next_event_id();
        for &idx in &self.curr_tree.on_anim_nodes {
            if self.curr_tree.nodes[idx].enabled.get_or(true) {
                self.event_queue
                    .push_back(QueuedEvent::new(idx, event_id, On::AnimationFrame, EventInfo::Animation(dt)));
            }
        }
    }

    pub fn queue_keyboard_event(&mut self, event: &KeyboardEvent) {
        let event_id = self.next_event_id();

        // Dispatch to the focused node
        let mut sent_to_root = false;
        if let Some(node) = &self.focused_node
            && let Some(&idx) = self.curr_tree.nid_map.get(node)
        {
            self.event_queue
                .push_back(QueuedEvent::new(idx, event_id, On::Keyboard, EventInfo::Keyboard(event.clone())));
            sent_to_root = idx == 0;
        }

        // Always dispatch to root
        if !sent_to_root {
            self.event_queue
                .push_back(QueuedEvent::new(0, event_id, On::Keyboard, EventInfo::Keyboard(event.clone())));
        }
    }

    pub fn queue_pointer_leave_event(&mut self) {
        self.temp.reset();
        self.pointer_delta = None;
        self.last_pointer_event = None;

        std::mem::swap(&mut self.curr_hot_indexes, &mut self.prev_hot_indexes);
        self.curr_hot_indexes.clear(); // nothing is hot because we're leaving the window

        self.queue_pointer_leave_enter_events(EventInfo::None);

        // Clear last pointer event because it's used for delta calculations
        self.last_pointer_event = None;
    }

    pub fn queue_pointer_wheel_event(&mut self, event: &PointerEvent) {
        self.queue_pointer_event(event, On::PointerWheel)
    }

    pub fn queue_pointer_move_event(&mut self, event: &PointerEvent) {
        self.queue_pointer_event(event, On::PointerMove)
    }

    pub fn queue_pointer_down_event(&mut self, event: &PointerEvent) {
        self.queue_pointer_event(event, On::PointerDown)
    }

    pub fn queue_pointer_up_event(&mut self, event: &PointerEvent) {
        self.queue_pointer_event(event, On::PointerUp)
    }

    fn queue_pointer_event(&mut self, event: &PointerEvent, event_type: On) {
        self.temp.reset();
        let last_event_pos = event.viewport_pos;
        if let Some(prev_event) = &self.last_pointer_event {
            self.pointer_delta = Some(event.viewport_pos - prev_event.viewport_pos);
        } else {
            self.pointer_delta = None;
        }
        self.last_pointer_event = Some(*event);

        std::mem::swap(&mut self.curr_hot_indexes, &mut self.prev_hot_indexes);
        self.update_hot_indexes_at(last_event_pos);

        let mut capture_idx: Option<usize> = None;

        // If captured, clamp hover to the captured node if the pointer is actually over it.
        if let Some(captured_nid) = self.capture {
            if let Some(&cap_idx) = self.curr_tree.nid_map.get(&captured_nid) {
                capture_idx = Some(cap_idx);

                let over = self.curr_hot_indexes.contains(&cap_idx);
                self.curr_hot_indexes.clear();
                if over {
                    self.curr_hot_indexes.push(cap_idx);
                }
            } else {
                // Captured node doesn't exist anymore
                self.capture = None;
            }
        }

        self.queue_pointer_leave_enter_events(EventInfo::Pointer(*event));

        let event_id = self.next_event_id();

        if let Some(idx) = capture_idx {
            // While captured, always dispatch to the captured node.
            self.event_queue
                .push_back(QueuedEvent::new(idx, event_id, event_type, EventInfo::Pointer(*event)));
        } else {
            // PointerDown, PointerMove, PointerUp, and PointerWheel are all sent to the frontmost nodes first
            for &idx in self.curr_hot_indexes.iter().rev() {
                self.event_queue
                    .push_back(QueuedEvent::new(idx, event_id, event_type, EventInfo::Pointer(*event)));
            }
        }
    }

    fn queue_pointer_leave_enter_events(&mut self, info: EventInfo) {
        let mut pointer_enter_nodes: BumpVec<usize> = BumpVec::with_capacity_in(self.curr_hot_indexes.len(), &self.temp);
        let mut pointer_leave_nodes: BumpVec<usize> = BumpVec::with_capacity_in(self.prev_hot_indexes.len(), &self.temp);

        sorted_iter_diff(self.curr_hot_indexes.iter(), self.prev_hot_indexes.iter(), &mut pointer_enter_nodes, &mut pointer_leave_nodes);

        // Hover pseudo-classes may affect styling.
        // Mark affected nodes dirty and ensure we run a draw pass.
        let mut dirtied = false;
        for &idx in pointer_enter_nodes.iter().chain(pointer_leave_nodes.iter()) {
            let style_flags = self.curr_tree.style_flags.get(idx).copied().unwrap_or(0);
            if (style_flags & css::HOVER_DIRTY) == 0 {
                continue;
            }
            self.curr_tree.dirty_roots.push(idx);
            dirtied = true;
        }
        if dirtied {
            // style pass will determine if layout should run
            self.phase.update(Phase::Draw);
        }

        // Never emit enter/leave while captured
        if let Some(captured_node) = &self.capture {
            if !self.curr_tree.nid_map.contains_key(captured_node) {
                // Captured node doesn't exist anymore
                self.capture = None;
            } else {
                return;
            }
        }

        // Normal enter/leave dispatch
        for &idx in pointer_leave_nodes.iter().rev() {
            let event_id = self.next_event_id();
            self.event_queue.push_back(QueuedEvent::new(idx, event_id, On::PointerLeave, info.clone()));
        }
        for &idx in pointer_enter_nodes.iter().rev() {
            let event_id = self.next_event_id();
            self.event_queue.push_back(QueuedEvent::new(idx, event_id, On::PointerEnter, info.clone()));
        }

        // Commit hot set changes so we don't queue the same events again
        if !pointer_enter_nodes.is_empty() || !pointer_leave_nodes.is_empty() {
            self.prev_hot_indexes.clear();
            self.prev_hot_indexes.extend(self.curr_hot_indexes.iter().copied());
        }
    }

    fn queue_lifecycle_events(&mut self) {
        self.temp.reset();
        let event_id = self.next_event_id();

        let mut create_nids: BumpVec<NodeId> = BumpVec::with_capacity_in(self.curr_tree.nid_sorted.len(), &self.temp);
        let mut destroy_nids: BumpVec<NodeId> = BumpVec::with_capacity_in(self.prev_tree.nid_sorted.len(), &self.temp);

        sorted_iter_diff(self.curr_tree.nid_sorted.iter(), self.prev_tree.nid_sorted.iter(), &mut create_nids, &mut destroy_nids);

        // On::Destroy
        for &nid in &destroy_nids {
            if let Some(&idx) = self.prev_tree.nid_map.get(&nid) {
                self.event_queue.push_back(QueuedEvent::new(idx, event_id, On::Destroy, EventInfo::None));
            }
        }

        // On::Create
        for &nid in &create_nids {
            if let Some(&idx) = self.curr_tree.nid_map.get(&nid) {
                self.event_queue.push_back(QueuedEvent::new(idx, event_id, On::Create, EventInfo::None));
            }
        }
    }

    pub fn dispatch_event_queue(&mut self, state: &mut S, handle: &H) -> Option<DispatchInfo> {
        if self.curr_tree.nodes.is_empty() {
            return None;
        }

        let start_active_node = self.active_node;
        let start_focused_node = self.focused_node;

        let mut dispatch_info = DispatchInfo::default();

        while let Some(qe) = self.event_queue.pop_front() {
            if qe.depth >= MAX_EVENT_DEPTH {
                error!("Event callback recursion depth exceeded MAX_EVENT_DEPTH ({MAX_EVENT_DEPTH})");
                continue;
            }

            // If disabled, don't dispatch any events to node, except for lifecycle events.
            if !matches!(qe.event_type, On::Create | On::Destroy) {
                // Note: only On::Destroy events use indices from prev_tree, so we're ok in this branch.
                if !self.curr_tree.nodes[qe.idx].enabled.get_or(true) {
                    continue;
                }
            }

            // Intercept AccessKit focus action and let the existing focus-change code at the end of the loop queue Blur/Focus events.
            let intercept_ax_focus =
                qe.event_type == On::AccessibilityAction && matches!(qe.event_info, EventInfo::AccessibilityAction(ref req) if req.action == Action::Focus);

            // ---------- Call Callback ----------

            let pointer_delta = if qe.event_type.is_pointer() { self.pointer_delta } else { None };

            let (has_callback, mut ctx) = if qe.event_type == On::Destroy {
                let mut ctx = EventCtx {
                    active_node: self.active_node,
                    captured_node: self.capture,
                    emit_change: false,
                    event_type: qe.event_type,
                    focus_direction: FocusDirection::None,
                    focused_node: self.focused_node,
                    handle,
                    id: self.prev_tree.nodes[qe.idx].nid,
                    idx: qe.idx,
                    info: qe.event_info,
                    is_enabled: self.prev_tree.nodes[qe.idx].enabled.get_or(true),
                    perf_info: &self.perf_info,
                    pointer_delta,
                    rect: &self.prev_tree.layout_cache[qe.idx],
                    stop_bubbling: false,
                    stop_window_close: false,
                    style: &self.prev_tree.style_cache[qe.idx],
                    translation_map: self.translation_map.clone(),
                    viewport_size: self.size,
                };

                (self.prev_tree.nodes[qe.idx].run_callbacks(qe.event_type, state, &mut ctx), ctx)
            } else {
                let mut ctx = EventCtx {
                    active_node: self.active_node,
                    captured_node: self.capture,
                    emit_change: false,
                    event_type: qe.event_type,
                    focus_direction: FocusDirection::None,
                    focused_node: self.focused_node,
                    handle,
                    id: self.curr_tree.nodes[qe.idx].nid,
                    idx: qe.idx,
                    info: qe.event_info,
                    is_enabled: self.curr_tree.nodes[qe.idx].enabled.get_or(true),
                    perf_info: &self.perf_info,
                    pointer_delta,
                    rect: &self.curr_tree.layout_cache[qe.idx],
                    stop_bubbling: false,
                    stop_window_close: false,
                    style: &self.curr_tree.style_cache[qe.idx],
                    translation_map: self.translation_map.clone(),
                    viewport_size: self.size,
                };

                if intercept_ax_focus {
                    // Focus the target node and skip user callback.
                    ctx.focused_node = self.curr_tree.nodes[qe.idx].nid;
                    (true, ctx)
                } else {
                    (self.curr_tree.nodes[qe.idx].run_callbacks(qe.event_type, state, &mut ctx), ctx)
                }
            };
            if !has_callback {
                continue;
            }

            if !intercept_ax_focus {
                dispatch_info.callback_count += 1;
            }

            self.active_node = ctx.active_node;

            // We cannot blindly set self.capture = ctx.captured_node, because a node (like a TextBox losing focus)
            // might try to release the capture (ctx.captured_node = None) even if it doesn't currently own it.
            if ctx.captured_node != self.capture {
                if let Some(new_capture) = ctx.captured_node {
                    if self.curr_tree.nid_map.contains_key(&new_capture) {
                        self.capture = Some(new_capture);
                        ctx.stop_propagation();
                    } else {
                        self.capture = None;
                    }
                } else if self.capture == ctx.id {
                    self.capture = None;
                }
            }

            // Pointer event callbacks can stop an event from bubbling up
            if ctx.stop_bubbling && qe.event_type.is_pointer() {
                dispatch_info.bubbling_stopped |= ctx.stop_bubbling;
                self.event_queue.retain(|event| event.event_id != qe.event_id);
            }

            if qe.event_type == On::WindowClose {
                dispatch_info.stop_window_close |= ctx.stop_window_close;
            }

            // ---------- Queue Secondary Events ----------

            if ctx.emit_change && qe.event_type != On::Destroy {
                // If an On::Change event handler emits a change, don't call itself
                if qe.event_type != On::Change && self.curr_tree.nodes[qe.idx].has_callback(On::Change) {
                    self.event_queue.push_back(QueuedEvent {
                        idx: qe.idx,
                        event_id: qe.event_id,
                        event_type: On::Change,
                        event_info: EventInfo::None,
                        depth: qe.depth + 1,
                    });
                } else {
                    // Search up tree for change event handler
                    let mut curr = self.curr_tree.nodes[qe.idx].parent;
                    while curr != usize::MAX {
                        if self.curr_tree.nodes[curr].has_callback(On::Change) {
                            self.event_queue.push_back(QueuedEvent {
                                idx: curr,
                                event_id: qe.event_id,
                                event_type: On::Change,
                                event_info: EventInfo::None,
                                depth: qe.depth + 1,
                            });
                            break;
                        }
                        curr = self.curr_tree.nodes[curr].parent;
                    }
                }
            }

            match ctx.focus_direction {
                FocusDirection::None => {}
                FocusDirection::Next => {
                    let len = self.curr_tree.nodes.len();
                    if let Some(focused_nid) = ctx.focused_node {
                        if let Some(&focused_idx) = self.curr_tree.nid_map.get(&focused_nid) {
                            for i in 1..len {
                                let idx = (focused_idx + i) % len;
                                if self.curr_tree.nodes[idx].has_callback(On::Focus) && self.curr_tree.nodes[idx].enabled.get_or(true) {
                                    ctx.focused_node = self.curr_tree.nodes[idx].nid;
                                    break;
                                }
                            }
                        }
                    } else {
                        for idx in 0..len {
                            if self.curr_tree.nodes[idx].has_callback(On::Focus) && self.curr_tree.nodes[idx].enabled.get_or(true) {
                                ctx.focused_node = self.curr_tree.nodes[idx].nid;
                                break;
                            }
                        }
                    }
                }
                FocusDirection::Previous => {
                    let len = self.curr_tree.nodes.len();
                    if let Some(focused_nid) = ctx.focused_node {
                        if let Some(&focused_idx) = self.curr_tree.nid_map.get(&focused_nid) {
                            for i in 1..len {
                                let idx = (focused_idx as isize - i as isize).rem_euclid(len as isize) as usize;
                                if self.curr_tree.nodes[idx].has_callback(On::Focus) && self.curr_tree.nodes[idx].enabled.get_or(true) {
                                    ctx.focused_node = self.curr_tree.nodes[idx].nid;
                                    break;
                                }
                            }
                        }
                    } else {
                        for idx in (0..len).rev() {
                            if self.curr_tree.nodes[idx].has_callback(On::Focus) && self.curr_tree.nodes[idx].enabled.get_or(true) {
                                ctx.focused_node = self.curr_tree.nodes[idx].nid;
                                break;
                            }
                        }
                    }
                }
            }

            if self.focused_node != ctx.focused_node {
                match (self.focused_node, ctx.focused_node) {
                    (Some(blur_nid), Some(focus_nid)) => {
                        if let Some(&idx) = self.curr_tree.nid_map.get(&blur_nid) {
                            self.event_queue.push_back(QueuedEvent {
                                idx,
                                event_id: qe.event_id,
                                event_type: On::Blur,
                                event_info: EventInfo::None,
                                depth: qe.depth + 1,
                            });
                        }
                        if let Some(&idx) = self.curr_tree.nid_map.get(&focus_nid) {
                            self.event_queue.push_back(QueuedEvent {
                                idx,
                                event_id: qe.event_id,
                                event_type: On::Focus,
                                event_info: EventInfo::None,
                                depth: qe.depth + 1,
                            });
                        }
                    }
                    (Some(blur_nid), None) => {
                        if let Some(&idx) = self.curr_tree.nid_map.get(&blur_nid) {
                            self.event_queue.push_back(QueuedEvent {
                                idx,
                                event_id: qe.event_id,
                                event_type: On::Blur,
                                event_info: EventInfo::None,
                                depth: qe.depth + 1,
                            });
                        }
                    }
                    (None, Some(focus_nid)) => {
                        if let Some(&idx) = self.curr_tree.nid_map.get(&focus_nid) {
                            self.event_queue.push_back(QueuedEvent {
                                idx,
                                event_id: qe.event_id,
                                event_type: On::Focus,
                                event_info: EventInfo::None,
                                depth: qe.depth + 1,
                            });
                        }
                    }
                    (None, None) => {}
                }

                self.focused_node = ctx.focused_node;
            }
        }

        // ---------- Update Phase Variable ----------

        let mut dirtied = false;

        if self.active_node != start_active_node {
            // Previously active loses :active, new active gains :active
            dirtied |= self.curr_tree.add_dirty_root_by_nid(start_active_node, css::ACTIVE_DIRTY);
            dirtied |= self.curr_tree.add_dirty_root_by_nid(self.active_node, css::ACTIVE_DIRTY);
        }

        if self.focused_node != start_focused_node {
            // Previously focused loses :focus, new focused gains :focus
            dirtied |= self.curr_tree.add_dirty_root_by_nid(start_focused_node, css::FOCUS_DIRTY);
            dirtied |= self.curr_tree.add_dirty_root_by_nid(self.focused_node, css::FOCUS_DIRTY);
        }

        if dirtied {
            // style pass will determine if layout should run
            self.phase.update(Phase::Draw);
        }

        let current_write_count = Registry::global().write_count();
        if self.last_write_count != current_write_count {
            self.last_write_count = current_write_count;

            if self.build_deps.any_changed() {
                self.phase = Phase::Build;
            } else if self.layout_deps.any_changed() {
                self.phase.update(Phase::Layout);
            } else if self.draw_deps.any_changed() || self.style_deps.any_changed() {
                self.phase.update(Phase::Draw);
            }
        }

        Some(dispatch_info)
    }

    /// [`Viewport::frame`] may queue events
    pub fn frame(&mut self, state: &S) -> &Scene {
        let start_time = Instant::now();

        if !self.event_queue.is_empty() {
            error!("Event queue must be empty before calling frame(). Dropped {} events.", self.event_queue.len());
            self.event_queue.clear();
        }

        let mut perf_info = PerfInfo::default();
        perf_info.frame_number = self.perf_info.frame_number + 1;
        perf_info.node_count = self.curr_tree.nodes.len();

        if self.phase == Phase::Idle {
            self.perf_info = perf_info;
            return &self.scene_cache;
        }

        self.temp.reset();

        // ---------- Build Phase ----------

        let mut did_build = false;
        if self.phase == Phase::Build || self.curr_tree.nodes.is_empty() {
            did_build = true;

            std::mem::swap(&mut self.prev_tree, &mut self.curr_tree);
            self.curr_tree.clear();

            let build_deps = std::mem::take(&mut self.build_deps).cleared().read_scope(|| {
                (self.view_callback)(state, &mut self.curr_tree);
            });
            self.build_deps = build_deps;

            self.curr_tree.finish();
            perf_info.node_count = self.curr_tree.nodes.len();

            if let Some(nid) = self.focused_node
                && !self.curr_tree.nid_map.contains_key(&nid)
            {
                self.focused_node = None;
            }

            if let Some(nid) = self.active_node
                && !self.curr_tree.nid_map.contains_key(&nid)
            {
                self.active_node = None;
            }

            if let Some(nid) = self.capture
                && !self.curr_tree.nid_map.contains_key(&nid)
            {
                self.capture = None;
            }

            // Re-create prev_hot_indexes to ensure pointer events will still fire correctly
            self.prev_hot_indexes.clear();
            for &idx in &self.curr_hot_indexes {
                if let Some(nid) = self.prev_tree.nodes[idx].nid
                    && let Some(&new_idx) = self.curr_tree.nid_map.get(&nid)
                {
                    self.prev_hot_indexes.push(new_idx);
                }
            }
        }
        let build_complete = Instant::now();
        perf_info.build_time = build_complete.duration_since(start_time);

        // ---------- Style Phase ----------

        if did_build {
            css::style_pre_pass(&self.temp, &mut self.curr_tree, &mut self.ancestor_classes);
        } else {
            // mark dirty any nodes with a changed 'enabled' status
            let mut dynamic_enabled_prev = std::mem::take(&mut self.curr_tree.dynamic_enabled_prev);
            for (idx, prev) in dynamic_enabled_prev.iter_mut() {
                if let UIParam::Dynamic(param) = &self.curr_tree.nodes[*idx].enabled {
                    let curr = param.get_or(true);
                    if curr != *prev {
                        *prev = curr;
                        self.curr_tree.add_dirty_root_by_idx(*idx, css::ENABLED_DIRTY);
                    }
                }
            }
            self.curr_tree.dynamic_enabled_prev = dynamic_enabled_prev;

            // mark dirty any nodes with changed dependencies for on_style callbacks
            for idx in &self.curr_tree.on_style_nodes {
                if let Some(deps) = self.curr_tree.on_style_deps.get_mut(idx)
                    && deps.any_changed_update()
                {
                    self.curr_tree.dirty_roots.push(*idx);
                }
            }
        }

        // The style pass always runs, and triggers a layout if needed.
        let mut changed_layout = false;
        let style_deps = std::mem::take(&mut self.style_deps).cleared().read_scope(|| {
            // mark the on_style deps as read so the style pass knows it still depends on those vars
            for idx in &self.curr_tree.on_style_nodes {
                if let Some(deps) = self.curr_tree.on_style_deps.get(idx) {
                    deps.mark_read();
                }
            }

            // this needs to run in the scope so that deps get registered on the first pass
            changed_layout = css::style_pass(
                &self.temp,
                &mut self.curr_tree,
                state,
                self.focused_node,
                self.active_node,
                &self.curr_hot_indexes,
                &mut self.ancestor_classes,
            );
        });
        self.style_deps = style_deps;

        let style_complete = Instant::now();
        perf_info.style_time = style_complete.duration_since(build_complete);

        // ---------- Layout Phase ----------

        let mut did_layout = false;
        if self.phase >= Phase::Layout || changed_layout || self.curr_tree.layout_cache.is_empty() {
            did_layout = true;

            let layout_deps = std::mem::take(&mut self.layout_deps).cleared().read_scope(|| {
                layout::layout(state, &self.temp, &mut self.curr_tree, self.size, self.scale, &self.translation_map);
            });
            self.layout_deps = layout_deps;

            if let Some(last_event) = &mut self.last_pointer_event {
                let last_event_pos = last_event.viewport_pos;
                self.update_hot_indexes_at(last_event_pos);
                let dirtied = self.mark_hot_dirty();

                // When we build the tree, we can't know what nodes are hot until after we calculate layout.
                // On build frames we do a second style pass when needed, and if that would effect layout, we layout again.
                // Without this, there would be a one-frame delay following rebuilds for pseudo-classes.
                // If client code were to require a Build every frame, then pseudo-classes would break.
                // So, Phase::Build frames will do selector matching and layout twice if needed,
                // but that should be rare, and layout is fast.
                if did_build && dirtied {
                    let mut should_layout = false;

                    // doesn't clear so that deps from previous pass aren't lost
                    let style_deps = std::mem::take(&mut self.style_deps).read_scope(|| {
                        should_layout = css::style_pass(
                            &self.temp,
                            &mut self.curr_tree,
                            state,
                            self.focused_node,
                            self.active_node,
                            &self.curr_hot_indexes,
                            &mut self.ancestor_classes,
                        );
                    });
                    self.style_deps = style_deps;

                    if should_layout {
                        let layout_deps = std::mem::take(&mut self.layout_deps).cleared().read_scope(|| {
                            layout::layout(state, &self.temp, &mut self.curr_tree, self.size, self.scale, &self.translation_map);
                        });
                        self.layout_deps = layout_deps;

                        // we don't mark dirty after this because queue_pointer_leave_enter_events
                        // will take care of it at the end of the frame.
                        self.update_hot_indexes_at(last_event_pos);

                        perf_info.layout_twice = true;
                    }
                }
            }
        }
        let layout_complete = Instant::now();
        perf_info.layout_time = layout_complete.duration_since(style_complete);

        // ---------- Draw Phase ----------

        self.scene_cache.reset();

        let draw_deps = std::mem::take(&mut self.draw_deps).cleared().read_scope(|| {
            draw::draw(
                &self.temp,
                state,
                &mut self.curr_tree,
                did_layout,
                &self.perf_info,
                self.scale,
                self.active_node,
                self.focused_node,
                self.translation_map.clone(),
                &mut self.scene_cache,
            );
        });
        self.draw_deps = draw_deps;

        let scene_complete = Instant::now();
        perf_info.scene_time = scene_complete.duration_since(layout_complete);

        // ---------- Cleanup ----------

        if self.phase == Phase::Build {
            self.queue_lifecycle_events();
        }
        self.phase = Phase::Idle;
        if let Some(info) = self.last_pointer_event {
            self.queue_pointer_leave_enter_events(EventInfo::Pointer(info)); // could modify self.phase
        }

        self.perf_info = perf_info;
        &self.scene_cache
    }
}
