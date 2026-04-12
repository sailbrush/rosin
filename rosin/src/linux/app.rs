use crate::linux::util;
use crate::linux::wayland::WaylandWindow;
use crate::prelude::*;
use pollster::FutureExt;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle};
use rosin_core::prelude::Viewport;
use rosin_core::vello::{self, AaSupport};
use rosin_core::wgpu;
use rosin_core::wgpu::ExperimentalFeatures;
use std::cell::RefCell;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::OnceLock;
use wayland_backend::client::ObjectId;
static _APP_STARTED: OnceLock<()> = OnceLock::new();

pub(crate) struct AppLauncher<S: Sync + 'static> {
    pub(crate) windows: Vec<WindowDesc<S>>,
    pub(crate) translation_map: Option<TranslationMap>,
    pub(crate) wgpu_config: WgpuConfig,
    pub(crate) state: Option<Rc<RefCell<S>>>,

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    hot_reloader: RefCell<Option<crate::mac::hot::HotReloader>>,
}

impl<S: Sync + 'static> AppLauncher<S> {
    pub fn new(window: WindowDesc<S>) -> Self {
        Self {
            windows: vec![window],
            translation_map: None,
            wgpu_config: WgpuConfig::default(),
            state: None,

            #[cfg(all(feature = "hot-reload", debug_assertions))]
            hot_reloader: RefCell::new(None),
        }
    }

    pub fn with_wgpu_config(mut self, config: WgpuConfig) -> Self {
        self.wgpu_config = config;
        self
    }

    pub fn add_window(mut self, window: WindowDesc<S>) -> Self {
        self.windows.push(window);
        self
    }
    // based on https://github.com/Smithay/client-toolkit/blob/master/examples/wgpu.rs
    // No hot-reload, no serde requirement
    #[cfg(not(all(feature = "hot-reload", debug_assertions)))]
    pub fn run(mut self, _state: S, _translation_map: TranslationMap) -> Result<(), LaunchError> {
        self.state = Some(Rc::new(RefCell::new(_state)));
        let way_conn = wayland_client::Connection::connect_to_env();
        use wayland_client::Proxy;

        use crate::linux::csd_frame::frame::FallbackFrame;
        use crate::linux::{handle::InputHandlerVars, wayland::RosinWaylandState, wayland::create_window_wayland};

        let conn = way_conn.unwrap();
        let (globals, event_queue) = wayland_client::globals::registry_queue_init(&conn).unwrap();
        let qh = event_queue.handle();

        let desc = self.windows[0].clone();
        let mut window = create_window_wayland(&desc, &globals, &qh);
        let raw_display_handle = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(NonNull::new(conn.backend().display_ptr() as *mut _).unwrap()));
        let raw_window_handle = RawWindowHandle::Wayland(WaylandWindowHandle::new(NonNull::new(window.surface.id().as_ptr() as *mut _).unwrap()));
        std::sync::Arc::<WaylandWindow>::get_mut(&mut window).unwrap().conn = Some(conn);
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: self.wgpu_config.backends,
            ..Default::default()
        });
        let adapter = match instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: self.wgpu_config.power_preference,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .block_on()
        {
            Ok(adapter) => adapter,
            Err(_e) => util::panic_and_print("GPU initialization failed".to_string()),
        };

        let (device, queue) = match adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("RosinDevice"),
                required_features: self.wgpu_config.features,
                required_limits: self.wgpu_config.limits.clone(),
                memory_hints: self.wgpu_config.memory_hints.clone(),
                trace: wgpu::Trace::Off,
                experimental_features: ExperimentalFeatures::disabled(),
            })
            .block_on()
        {
            Ok((device, queue)) => (device, queue),
            Err(_e) => util::panic_and_print("GPU initialization failed".to_string()),
        };

        let compositor = Compositor {
            blitter: RefCell::new(None),
            custom: RefCell::new(None),
        };
        let gpu_ctx = Rc::new(GpuCtx {
            instance,
            adapter,
            device,
            queue,
            compositor,
        });

        let wgpu_surface: wgpu::Surface<'static> = unsafe {
            gpu_ctx
                .instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle,
                    raw_window_handle,
                })
                .unwrap()
        };
        let vello_texture = {
            let view_formats = [];
            gpu_ctx.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: desc.size.width as u32,
                    height: desc.size.height as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: view_formats.as_slice(),
            })
        };
        let viewport: Viewport<S, crate::prelude::WindowHandle> =
            Viewport::new(desc.viewfn.func, desc.size, rosin_core::kurbo::Vec2 { x: 1.0f64, y: 1.0f64 }, _translation_map);

        let wh = crate::prelude::WindowHandle(crate::linux::handle::WindowHandle {
            wayland_handle: Some(window),
            input_handler: std::sync::Arc::new(rosin_core::parking_lot::RwLock::new(InputHandlerVars { id: None, handler: None, file_dialog_result: None, dialog_id: None })),
        });
        let vello_renderer = {
            let renderer = match vello::Renderer::new(
                &gpu_ctx.device,
                vello::RendererOptions {
                    use_cpu: false,
                    antialiasing_support: AaSupport::all(),
                    num_init_threads: None,
                    pipeline_cache: None,
                },
            ) {
                Ok(r) => r,
                Err(_e) => util::panic_and_print("GPU initialization failed".to_string()),
            };

            Rc::new(RefCell::new(renderer))
        };
        use crate::kurbo::Vec2;

        let mut frame = if wh.0.wayland_handle.as_ref().unwrap().toplevel_decoration.is_none() {
            Some(FallbackFrame::new(wh.0.wayland_handle.as_ref().unwrap().as_ref(), qh).expect("msg"))
        } else {
            None
        };
        if frame.is_some() {
            frame.as_mut().unwrap().update_state(wayland_csd_frame::WindowState::ACTIVATED);
        }
        let mut rosin_window: RosinWaylandState<S> = RosinWaylandState {
            exit: false,
            width: desc.size.width as u32,
            height: desc.size.height as u32,
            gpu_ctx,
            vello_renderer,
            tex_to_render: vello_texture,
            surface: wgpu_surface,
            viewport,
            app_state: self.state.unwrap(),
            window_handle: wh,
            last_mouse_pos: Vec2::new(0.0, 0.0),
            wgpufn: desc.wgpufn,
            pressed_modifiers: 0,
            fallback_frame: frame,
            last_surface_id: ObjectId::null(),
            seat: None,
        };
        use wayland_csd_frame::DecorationsFrame;
        let _ = rosin_window.run_loop(event_queue);
        Ok(())
    }

    // Yes hot-reload, yes serde requirement
    #[cfg(all(feature = "hot-reload", debug_assertions))]
    pub fn run(mut self, mut _state: S, _translation_map: TranslationMap) -> Result<(), LaunchError>
    where
        S: serde::Serialize + serde::de::DeserializeOwned + crate::typehash::TypeHash + 'static,
    {
        // TODO
        Ok(())
    }
}
