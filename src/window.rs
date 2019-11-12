use std::{collections::HashMap, error::Error};

use cssparser::Color;
use euclid::{TypedPoint2D, TypedRect, TypedSize2D};
use gleam::gl;
use glutin::EventsLoop;
use webrender::api::*;
use webrender::DebugFlags;
use webrender::ShaderPrecacheFlags;

use crate::app::*;
use crate::dom::*;
use crate::layout::*;
use crate::style::*;
use crate::system::*;
use crate::view::*;

pub struct WindowBuilder<T> {
    pub title: &'static str,
    pub width: f64,
    pub height: f64,
    pub view: Option<View<T>>,
    pub event_callback: Option<EventCallback>,
}

impl<T> Default for WindowBuilder<T> {
    fn default() -> Self {
        WindowBuilder {
            title: "Rosin Window",
            width: 800.0,
            height: 600.0,
            view: None,
            event_callback: None,
        }
    }
}

impl<T> WindowBuilder<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_title(mut self, title: &'static str) -> Self {
        self.title = title;
        self
    }

    pub fn with_width(mut self, width: f64) -> Self {
        self.width = width;
        self
    }

    pub fn with_height(mut self, height: f64) -> Self {
        self.height = height;
        self
    }

    pub fn with_view(mut self, view: View<T>) -> Self {
        self.view = Some(view);
        self
    }

    pub fn with_event_callback(mut self, event_callback: EventCallback) -> Self {
        self.event_callback = Some(event_callback);
        self
    }
}

pub struct RosinWindow<T> {
    pub view: View<T>,
    pub cached_dom: Dom<T>,

    pub context_id: ContextId,
    pub renderer: Option<webrender::Renderer>,
    pub api: webrender::webrender_api::RenderApi,
    pub document_id: webrender::webrender_api::DocumentId,
    pub epoch: webrender::webrender_api::Epoch,
    pub pipeline_id: webrender::webrender_api::PipelineId,
    pub layout_size: webrender::euclid::TypedSize2D<f32, LayoutPixel>,
    pub framebuffer_size:
        webrender::euclid::TypedSize2D<i32, webrender::webrender_api::DevicePixel>,
}

pub struct WindowManager<T> {
    pub window_map: HashMap<glutin::WindowId, RosinWindow<T>>,
    pub context_mgr: ContextManager,
}

impl<T> Default for WindowManager<T> {
    fn default() -> Self {
        WindowManager {
            window_map: HashMap::default(),
            context_mgr: ContextManager::default(),
        }
    }
}

impl<T> WindowManager<T> {
    pub fn is_empty(&self) -> bool {
        self.window_map.is_empty()
    }

    pub fn create(
        &mut self,
        builder: WindowBuilder<T>,
        events_loop: &EventsLoop,
    ) -> Result<glutin::WindowId, Box<dyn Error>> {
        // Create Window & OpenGL context
        let window_builder = glutin::WindowBuilder::new()
            .with_title(builder.title)
            .with_multitouch()
            .with_dimensions(glutin::dpi::LogicalSize::new(builder.width, builder.height));
        let context = glutin::ContextBuilder::new().build_windowed(window_builder, events_loop)?;
        let context = unsafe { context.make_current().unwrap() };
        let device_pixel_ratio = context.window().get_hidpi_factor() as f32;

        let gl = match context.get_api() {
            glutin::Api::OpenGl => unsafe {
                gl::GlFns::load_with(|symbol| context.get_proc_address(symbol) as *const _)
            },
            glutin::Api::OpenGlEs => unsafe {
                gl::GlesFns::load_with(|symbol| context.get_proc_address(symbol) as *const _)
            },
            glutin::Api::WebGl => unimplemented!(),
        };

        let window_id = context.window().id();

        // Create webrender config
        let debug_flags = DebugFlags::ECHO_DRIVER_MESSAGES;
        let opts = webrender::RendererOptions {
            precache_flags: ShaderPrecacheFlags::EMPTY,
            device_pixel_ratio,
            clear_color: Some(ColorF::new(0.0, 0.0, 0.0, 1.0)),
            debug_flags,
            ..webrender::RendererOptions::default()
        };
        let framebuffer_size = {
            let size = context
                .window()
                .get_inner_size()
                .unwrap()
                .to_physical(f64::from(device_pixel_ratio));
            DeviceIntSize::new(size.width as i32, size.height as i32)
        };

        let context_id = self
            .context_mgr
            .insert(ContextCurrentWrapper::PossiblyCurrent(
                ContextWrapper::Windowed(context),
            ));

        // Init webrender
        let notifier = Box::new(Notifier::new(events_loop.create_proxy()));
        let (renderer, sender) =
            webrender::Renderer::new(gl.clone(), notifier, opts, None).unwrap();
        let api = sender.create_api();
        let document_id = api.add_document(framebuffer_size, 0);

        let epoch = Epoch(0);
        let pipeline_id = PipelineId(0, 0);

        let layout_size = framebuffer_size.to_f32() / euclid::TypedScale::new(device_pixel_ratio);

        self.window_map.insert(
            window_id,
            RosinWindow {
                view: builder
                    .view
                    .expect("[Rosin] Tried to create window without a view"),
                cached_dom: Dom::div(),
                context_id,
                renderer: Some(renderer),
                api,
                document_id,
                epoch,
                pipeline_id,
                layout_size,
                framebuffer_size,
            },
        );
        Ok(window_id)
    }

    pub fn close(&mut self, window_id: glutin::WindowId) {
        let mut window = self
            .window_map
            .remove(&window_id)
            .expect("[Rosin] Attempted to remove invalid window.");
        window.renderer.take().unwrap().deinit();
        self.context_mgr.remove(window.context_id);
    }

    pub fn resize(&mut self, window_id: glutin::WindowId, logical_size: glutin::dpi::LogicalSize) {
        let mut window = self
            .window_map
            .get_mut(&window_id)
            .expect("[Rosin] Attempted to resize invalid window.");
        let context = self
            .context_mgr
            .get_current(window.context_id)
            .unwrap()
            .windowed();
        let dpi_factor = context.window().get_hidpi_factor();
        let physical_size = logical_size.to_physical(dpi_factor);
        context.resize(physical_size);

        let framebuffer_size = DeviceIntSize::new(physical_size.width as i32, physical_size.height as i32);
        window.framebuffer_size = framebuffer_size;
        window.layout_size = framebuffer_size.to_f32() / euclid::TypedScale::new(dpi_factor as f32);
    }

    pub fn draw(
        &mut self,
        window_id: glutin::WindowId,
        dom: &Dom<T>,
        stylesheet: &Stylesheet,
    ) -> Result<(), Box<dyn Error>> {
        let window = self.window_map.get_mut(&window_id).unwrap();
        let styles = stylesheet.style(&dom);
        let layouts = Layout::solve(&dom, &styles, window.framebuffer_size.to_tuple());

        // Build displaylist from dom
        let mut builder = DisplayListBuilder::new(window.pipeline_id, window.layout_size);

        for (id, _) in dom.arena.iter().enumerate() {
            let style = styles[id];
            let layout = layouts[id];

            let background_color = match style.background_color {
                crate::style::PropertyValue::Exact(css_color) => match css_color {
                    Color::RGBA(color) => ColorF::new(
                        color.red as f32 / 255.0,
                        color.green as f32 / 255.0,
                        color.blue as f32 / 255.0,
                        color.alpha as f32 / 255.0,
                    ),
                    _ => ColorF::new(0.0, 0.0, 0.0, 0.0),
                },
                _ => ColorF::new(0.0, 0.0, 0.0, 0.0),
            };

            builder.push_rect(
                &LayoutPrimitiveInfo::new(TypedRect::new(
                    TypedPoint2D::new(layout.position.x, layout.position.y),
                    TypedSize2D::new(layout.size.width, layout.size.height),
                )),
                &SpaceAndClipInfo::root_scroll(window.pipeline_id),
                background_color,
            );
        }
        builder.pop_stacking_context();

        // Send displaylist to webrender
        let windowed_context = self.context_mgr.get_current(window.context_id).unwrap();
        let mut txn = Transaction::new();
        txn.set_display_list(
            window.epoch,
            Some(ColorF::new(0.0, 0.0, 0.0, 1.0)),
            window.layout_size,
            builder.finalize(),
            true,
        );
        txn.set_root_pipeline(window.pipeline_id);
        txn.generate_frame();
        window.api.send_transaction(window.document_id, txn);
        let renderer = window.renderer.as_mut().unwrap();
        renderer.update();
        renderer.render(window.framebuffer_size).unwrap();
        windowed_context.windowed().swap_buffers().unwrap();

        Ok(())
    }
}
