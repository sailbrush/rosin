use crate::alloc::Alloc;
use crate::prelude::*;
use crate::{alloc::Scope, draw, layout, layout::Layout, tree::*};

use std::error::Error;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use bumpalo::{collections::Vec as BumpVec, Bump};
use druid_shell::piet::Piet;

pub struct RosinWindow<S: 'static, H> {
    sheet_loader: Arc<Mutex<SheetLoader>>,
    view: ViewCallback<S>,
    size: (f32, f32),
    handle: Option<H>,
    phase: Phase,
    tree_cache: Option<Scope<BumpVec<'static, ArrayNode<S>>>>,
    layout_cache: Option<Scope<BumpVec<'static, Layout>>>,
    alloc: Rc<Alloc>,
    temp: Bump,
}

impl<S, H> RosinWindow<S, H> {
    pub fn new(sheet_loader: Arc<Mutex<SheetLoader>>, view: ViewCallback<S>, size: (f32, f32)) -> Self {
        Self {
            sheet_loader,
            view,
            size,
            handle: None,
            phase: Phase::Build,
            tree_cache: None,
            layout_cache: None,
            alloc: Rc::new(Alloc::default()),
            temp: Bump::new(),
        }
    }

    pub fn set_handle(&mut self, handle: H) {
        self.handle = Some(handle)
    }

    pub fn get_handle_ref(&self) -> Option<&H> {
        self.handle.as_ref()
    }

    pub fn get_handle_mut(&mut self) -> Option<&mut H> {
        self.handle.as_mut()
    }

    pub fn reset_cache(&mut self) {
        self.layout_cache = None;
        self.tree_cache = None;
        self.alloc.reset().expect("[Rosin] Failed to reset cache");
    }

    pub fn update_phase(&mut self, new_phase: Phase) {
        self.phase = self.phase.max(new_phase);
    }

    pub fn size(&mut self, new_size: (f32, f32)) {
        self.size.0 = new_size.0;
        self.size.1 = new_size.1;
        self.update_phase(Phase::Layout);
    }

    pub fn set_view(&mut self, new_view: ViewCallback<S>) {
        self.view = new_view;
        self.phase = Phase::Build;
    }

    pub fn do_anim_frame(&mut self, state: &mut S) {
        todo!();
    }

    pub fn is_idle(&self) -> bool {
        self.phase == Phase::Idle
    }

    pub fn get_alloc(&self) -> Rc<Alloc> {
        self.alloc.clone()
    }

    pub fn click(&mut self, state: &mut S, ctx: &mut EventCtx, position: (f32, f32)) {
        if let (Some(tree), Some(layout)) = (&mut self.tree_cache, &mut self.layout_cache) {
            let id = layout::hit_test(tree.borrow(), layout.borrow_mut(), (position.0 as f32, position.1 as f32));
            let test = tree.borrow_mut()[id].trigger(On::MouseDown, state, ctx);
            self.update_phase(test);
        }
    }

    pub fn draw(&mut self, state: &S, piet: &mut Piet<'_>) -> Result<(), Box<dyn Error>> {
        Alloc::set_thread_local_alloc(Some(self.alloc.clone()));
        let alloc = self.alloc.clone();

        // Reset scratch allocator
        self.temp.reset();

        // ---------- Build Phase ----------
        if self.phase == Phase::Build || self.tree_cache.is_none() {
            self.reset_cache();

            // Reset counter so we can track how many nodes are allocated
            alloc.reset_counter();

            // SAFETY: This is safe because we panic if client code breaks scope()'s contract
            let mut tree = unsafe {
                // Run the view function
                alloc.scope(|| (self.view)(state).finish().unwrap())
            };

            // Panic if the view function didn't return the number of nodes we expected
            assert!(alloc.get_counter() == tree.borrow().len(), "[Rosin] Nodes missing");

            self.sheet_loader.lock().unwrap().apply_style(tree.borrow_mut());
            self.tree_cache = Some(tree);
        }

        let tree: &mut BumpVec<ArrayNode<S>> = self.tree_cache.as_mut().unwrap().borrow_mut();

        // Stash default styles, and run style callbacks
        let mut default_styles: BumpVec<(usize, Style)> = BumpVec::new_in(&self.temp);
        if self.phase != Phase::Idle {
            for (id, node) in tree.iter_mut().enumerate() {
                if let Some(modify_style) = &mut node.style_callback {
                    default_styles.push((id, node.style.clone()));
                    modify_style(state, &mut node.style);
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
        }

        let layout: &BumpVec<Layout> = self.layout_cache.as_ref().unwrap().borrow();

        // ---------- Draw Phase ----------
        // TODO - If phase == Idle, re-issue commands from last frame
        draw::draw(state, tree, layout, piet);

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
