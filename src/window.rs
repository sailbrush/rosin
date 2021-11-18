use crate::layout::hit_test;
use crate::libloader::LibLoader;
use crate::prelude::*;
use crate::{alloc::Scope, geometry::Size, layout::*, render, tree::*};

use std::error::Error;

use bumpalo::{collections::Vec as BumpVec, Bump};
use femtovg::{renderer::OpenGl, Canvas};
use glutin::dpi::PhysicalPosition;
use glutin::{
    dpi::{LogicalSize, PhysicalSize},
    event_loop::EventLoopWindowTarget,
    window::{WindowBuilder, WindowId},
    PossiblyCurrent, WindowedContext,
};

// TODO - Just re-export winit types for window creation / events
/// A description of a window.
pub struct WindowDesc<T: 'static> {
    pub(crate) builder: WindowBuilder,
    pub(crate) view: View<T>,
}

impl<T> WindowDesc<T> {
    pub fn new(view: View<T>) -> Self {
        Self {
            builder: WindowBuilder::new(),
            view,
        }
    }

    pub fn with_title<S: Into<String>>(mut self, title: S) -> Self {
        self.builder = self.builder.with_title(title);
        self
    }

    pub fn with_size(mut self, width: f64, height: f64) -> Self {
        self.builder = self.builder.with_inner_size(LogicalSize::new(width, height));
        self
    }
}

pub(crate) struct RosinWindow<T: 'static> {
    windowed_context: WindowedContext<PossiblyCurrent>,
    canvas: Canvas<OpenGl>,
    view: View<T>,
    stage: Stage,
    tree_cache: Option<Scope<BumpVec<'static, ArrayNode<T>>>>,
    layout_cache: Option<Scope<BumpVec<'static, Layout>>>,
    temp: Bump,
    font_table: Vec<(u32, femtovg::FontId)>,
}

impl<T> RosinWindow<T> {
    pub fn new(desc: WindowDesc<T>, event_loop: &EventLoopWindowTarget<()>) -> Result<Self, Box<dyn Error>> {
        // TODO - handle errors better
        let windowed_context = unsafe {
            glutin::ContextBuilder::new()
                .build_windowed(desc.builder, event_loop)?
                .make_current()
                .unwrap()
        };
        let window_size = windowed_context.window().inner_size();

        let renderer = OpenGl::new(|s| windowed_context.get_proc_address(s) as *const _).expect("[Rosin] Cannot create renderer");
        let mut canvas = Canvas::new(renderer).expect("[Rosin] Cannot create canvas");
        canvas.set_size(
            window_size.width as u32,
            window_size.height as u32,
            windowed_context.window().scale_factor() as f32,
        );

        Ok(Self {
            windowed_context,
            canvas,
            view: desc.view,
            stage: Stage::Build,
            tree_cache: None,
            layout_cache: None,
            temp: Bump::new(),
            font_table: Vec::new(),
        })
    }

    fn reset_cache(&mut self, _loader: &LibLoader) {
        self.layout_cache = None;
        self.tree_cache = None;
        A.with(|a| a.reset().expect("[Rosin] Failed to reset cache"));

        #[cfg(all(debug_assertions, feature = "hot-reload"))]
        {
            let reset: fn() -> Result<(), ()> = *_loader.get(b"_rosin_reset_alloc").unwrap();
            reset().expect("[Rosin] Hot-reload: Failed to reset cache");
        }
    }

    pub fn id(&self) -> WindowId {
        self.windowed_context.window().id()
    }

    pub fn add_font_bytes(&mut self, id: u32, data: &[u8]) -> Result<(), Box<dyn Error>> {
        let font_id = self.canvas.add_font_mem(data)?;
        self.font_table.push((id, font_id));
        Ok(())
    }

    pub fn update_stage(&mut self, new_stage: Stage) {
        self.stage = self.stage.max(new_stage);
        if new_stage != Stage::Idle {
            self.windowed_context.window().request_redraw();
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.update_stage(Stage::Layout);

            self.windowed_context.resize(new_size);
        }
    }

    pub fn click(&mut self, state: &mut T, ctx: &mut EventCtx, position: PhysicalPosition<f64>) -> Stage {
        if let (Some(tree), Some(layout)) = (&mut self.tree_cache, &mut self.layout_cache) {
            let id = hit_test(layout.borrow_mut(), (position.x as f32, position.y as f32));
            tree.borrow_mut()[id].trigger(On::MouseDown, state, ctx)
        } else {
            Stage::Idle
        }
    }

    pub fn redraw(&mut self, state: &T, stylesheet: &Stylesheet, loader: &LibLoader) -> Result<(), Box<dyn Error>> {
        // Reset scratch allocator
        self.temp.reset();

        // Get window size and dpi
        let window_size = self.windowed_context.window().inner_size();
        let dpi_factor = self.windowed_context.window().scale_factor();

        // ---------- Rebuild window tree ----------
        if self.stage == Stage::Build || self.tree_cache.is_none() {
            self.reset_cache(loader);

            #[cfg(not(all(debug_assertions, feature = "hot-reload")))]
            let (mut tree, node_count) = {
                // Reset NODE_COUNT so we can track how many nodes are allocated
                NODE_COUNT.with(|c| c.set(0));

                // SAFETY: This is safe because we panic if client code breaks scope()'s contract
                let tree = A.with(|a| unsafe {
                    // Load and run the view function
                    a.scope(|| self.view.get(loader)(state).finish().unwrap())
                });

                (tree, NODE_COUNT.with(|c| c.get()))
            };

            #[cfg(all(debug_assertions, feature = "hot-reload"))]
            let (mut tree, node_count) = {
                // Reset NODE_COUNT so we can track how many nodes are allocated
                loader.get::<fn()>(b"_rosin_reset_node_count").unwrap()();

                // Manually begin a scope on dylib's allocator
                loader.get::<fn()>(b"_rosin_begin_alloc").unwrap()();

                // SAFETY: This is safe because we panic if client code breaks scope()'s contract
                let tree = A.with(|a| unsafe {
                    // Load and run the view function
                    a.scope(|| self.view.get(loader)(state).finish().unwrap())
                });

                // Manually end the dylib's scope
                loader.get::<fn()>(b"_rosin_end_alloc").unwrap()();

                (tree, loader.get::<fn() -> usize>(b"_rosin_get_node_count").unwrap()())
            };

            // Panic if the view function didn't return the number of nodes we expected
            assert!(node_count == tree.borrow().len(), "[Rosin] Nodes missing");

            stylesheet.style(&mut tree.borrow_mut());
            self.tree_cache = Some(tree);
        }

        let tree: &mut BumpVec<ArrayNode<T>> = &mut self.tree_cache.as_mut().unwrap().borrow_mut();

        // Stash default styles, and run style callbacks
        let mut default_styles: BumpVec<(usize, Style)> = BumpVec::new_in(&self.temp);
        if self.stage != Stage::Idle {
            for (id, node) in tree.iter_mut().enumerate() {
                if let Some(modify_style) = &mut node.style_on_draw {
                    default_styles.push((id, node.style.clone()));
                    modify_style(state, &mut node.style);
                }
            }
        }

        // ---------- Recalculate layout ----------
        if self.stage >= Stage::Layout || self.layout_cache.is_none() {
            if self.layout_cache.is_none() {
                let new_layout = A.with(|a| unsafe {
                    // SAFETY: This is safe because we meet scope()'s requirements
                    a.scope(|| A.with(|a| a.vec_capacity(tree.len())))
                });

                self.layout_cache = Some(new_layout);
            }

            let mut layout = self.layout_cache.as_mut().unwrap().borrow_mut();

            layout.clear();
            for _ in 0..tree.len() {
                layout.push(Layout::default());
            }

            let layout_size = Size {
                width: window_size.width as f32,
                height: window_size.height as f32,
            };
            build_layout(&self.temp, tree, layout_size, &mut layout);
        }

        let layout: &BumpVec<Layout> = self.layout_cache.as_ref().unwrap().borrow();

        // ---------- Render ----------
        // TODO - If stage == Idle, re-issue commands from last frame
        self.canvas
            .set_size(window_size.width as u32, window_size.height as u32, dpi_factor as f32);
        self.canvas
            .clear_rect(0, 0, window_size.width as u32, window_size.height as u32, femtovg::Color::black());

        render::render(state, tree, layout, &mut self.canvas, &self.font_table);

        self.canvas.flush();
        self.windowed_context.swap_buffers().unwrap();

        // ---------- Cleanup ----------
        self.stage = Stage::Idle;

        // Restore default styles
        for (id, style) in default_styles {
            tree[id].style = style;
        }

        Ok(())
    }
}
