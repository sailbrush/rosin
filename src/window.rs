use crate::prelude::*;
use crate::{geometry::Size, layout::*, libloader::LibLoader, render, tree::*};

use std::{error::Error, mem, ptr::NonNull};

use bumpalo::{collections::Vec as BumpVec, Bump};
use femtovg::{renderer::OpenGl, Canvas};
use glutin::{
    dpi::{LogicalSize, PhysicalSize},
    event_loop::EventLoopWindowTarget,
    window::{WindowBuilder, WindowId},
    PossiblyCurrent, WindowedContext,
};

// TODO Just re-export winit types for window creation / events
pub struct WindowDesc<T> {
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

// A struct with the same size as BumpVec used to erase lifetimes
// A BumpVec that has been orphaned from its allocator
struct OrphanVec {
    _a: NonNull<()>,
    _b: NonNull<()>,
    _c: NonNull<()>,
    _d: NonNull<()>,
}

impl OrphanVec {
    pub unsafe fn orphan<U>(vec: BumpVec<U>) -> OrphanVec {
        mem::transmute::<BumpVec<U>, OrphanVec>(vec)
    }

    pub unsafe fn adopt<U>(&self) -> &BumpVec<U> {
        &*(self as *const OrphanVec as *const BumpVec<U>)
    }

    pub unsafe fn adopt_mut<U>(&mut self) -> &mut BumpVec<U> {
        &mut *(self as *mut OrphanVec as *mut BumpVec<U>)
    }
}

pub(crate) struct RosinWindow<T> {
    windowed_context: WindowedContext<PossiblyCurrent>,
    canvas: Canvas<OpenGl>,
    view: View<T>,
    stage: Stage,
    alloc: Alloc,
    tree_cache: Option<OrphanVec>,
    layout_cache: Option<OrphanVec>,
    temp: Bump,
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

        let renderer =
            OpenGl::new(|s| windowed_context.get_proc_address(s) as *const _).expect("[Rosin] Cannot create renderer");
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
            alloc: Alloc::default(),
            tree_cache: None,
            layout_cache: None,
            temp: Bump::new(),
        })
    }

    fn reset_cache(&mut self) {
        self.layout_cache = None;
        self.tree_cache = None;
        self.alloc.bump.reset();
    }

    pub fn id(&self) -> WindowId {
        self.windowed_context.window().id()
    }

    pub fn borrow_tree_cache(&self) -> Option<&BumpVec<ArrayNode<T>>> {
        // SAFETY: The returned borrow is guaranteed to remain valid because
        // it points to heap allocated memory that requires &mut to clear
        Some(unsafe { self.tree_cache.as_ref()?.adopt() })
    }

    pub fn borrow_layout_cache(&self) -> Option<&BumpVec<Layout>> {
        // SAFETY: The returned borrow is guaranteed to remain valid because
        // it points to heap allocated memory that requires &mut to clear
        Some(unsafe { self.layout_cache.as_ref()?.adopt() })
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

    pub fn redraw(&mut self, state: &T, stylesheet: &Stylesheet, loader: &Option<LibLoader>) -> Result<(), Box<dyn Error>> {
        // Reset scratch allocator
        self.temp.reset();

        // Get window size and dpi
        let window_size = self.windowed_context.window().inner_size();
        let dpi_factor = self.windowed_context.window().scale_factor();

        // ---------- Rebuild window tree ----------
        if self.stage == Stage::Build || self.tree_cache.is_none() {
            self.reset_cache();
            let mut tree = self.view.get(loader)(&state, &self.alloc).finish(&self.alloc).unwrap();
            stylesheet.style(&mut tree);
            // SAFETY: This is needed to store self references, which allows us to retain bump allocated data between redraws
            self.tree_cache = Some(unsafe { OrphanVec::orphan(tree) });
        }

        let tree: &mut BumpVec<ArrayNode<T>> = unsafe { self.tree_cache.as_mut().unwrap().adopt_mut() };

        // Stash default styles, and run style callbacks
        let mut default_styles: BumpVec<(usize, Style)> = BumpVec::new_in(&self.temp);
        if self.stage != Stage::Idle {
            for (id, node) in tree.iter_mut().enumerate() {
                if let Some(modify_style) = node.style_on_draw {
                    default_styles.push((id, node.style.clone()));
                    modify_style(state, &mut node.style);
                }
            }
        }

        // ---------- Recalculate layout ----------
        if self.stage >= Stage::Layout || self.layout_cache.is_none() {
            if self.layout_cache.is_none() {
                let new_layout: BumpVec<Layout> = BumpVec::with_capacity_in(tree.len(), &self.alloc.bump);
                // SAFETY: This is needed to store self references, which allows us to retain bump allocated data between redraws
                self.layout_cache = Some(unsafe { OrphanVec::orphan(new_layout) });
            }

            let layout = unsafe { self.layout_cache.as_mut().unwrap().adopt_mut() };

            layout.clear();
            for _ in 0..tree.len() {
                layout.push(Layout::default());
            }

            let layout_size = Size {
                width: window_size.width as f32,
                height: window_size.height as f32,
            };
            build_layout(&self.temp, &tree, layout_size, layout);
        }

        let layout: &BumpVec<Layout> = unsafe { self.layout_cache.as_ref().unwrap().adopt() };

        //println!("BEGIN TREE ------------------------------------------");
        //let test: Vec<(&str, &Layout)> = tree.into_iter().map(|item| item.classes[0]).zip(layout.into_iter()).collect();
        //dbg!(test);
        //println!("END TREE ------------------------------------------");

        // ---------- Render ----------
        // TODO - If stage == Idle, re-issue commands from last frame
        self.canvas
            .set_size(window_size.width as u32, window_size.height as u32, dpi_factor as f32);
        self.canvas.clear_rect(
            0,
            0,
            window_size.width as u32,
            window_size.height as u32,
            femtovg::Color::black(),
        );

        render::render(tree, layout, &mut self.canvas);

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
