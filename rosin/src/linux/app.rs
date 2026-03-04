use crate::linux::wayland_state::RosinWaylandWindow;
use crate::prelude::*;
use pollster::FutureExt;
use rosin_core::wgpu::{self, ExperimentalFeatures};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    registry::RegistryState,
    seat::SeatState,
    shell::xdg::{XdgShell, window::WindowDecorations},
};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::OnceLock;
use wayland_client::Connection;
use wayland_client::QueueHandle;
use wayland_client::globals::registry_queue_init;
use wayland_client::Proxy;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle};
use wayland_backend::client::ObjectId;
use crate::linux::util::panic_and_print;
use rosin_core::vello::{self, AaSupport};
use std::ptr::NonNull;
static _APP_STARTED: OnceLock<()> = OnceLock::new();

pub(crate) struct AppLauncher<S: Sync + 'static> {
    windows: Vec<WindowDesc<S>>,
    _translation_map: Option<TranslationMap>,
    wgpu_config: WgpuConfig,
    _state: Option<Rc<RefCell<S>>>,

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    hot_reloader: RefCell<Option<crate::mac::hot::HotReloader>>,
}

impl<S: Sync + 'static> AppLauncher<S> {
    pub fn new(window: WindowDesc<S>) -> Self {
        Self {
            windows: vec![window],
            _translation_map: None,
            wgpu_config: WgpuConfig::default(),
            _state: None,

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
    pub fn run(self, _state: S, _translation_map: TranslationMap) -> Result<(), LaunchError> {
        let conn = Connection::connect_to_env().unwrap();
        let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
        let qh: QueueHandle<RosinWaylandWindow> = event_queue.handle();

        let compositor_state = CompositorState::bind(&globals, &qh).expect("wl_compositor not available");
        let xdg_shell_state = XdgShell::bind(&globals, &qh).expect("xdg shell not available");
        //implement multi-window later?
        //for desc in self.windows
        {
            let desc = &self.windows[0];
            let surface = compositor_state.create_surface(&qh);
            let window = xdg_shell_state.create_window(surface, WindowDecorations::RequestServer, &qh);
            window.set_title(desc.title.clone().unwrap().deref());
            window.set_app_id("rosin.default.id");
            window.set_min_size(Some((desc.min_size.unwrap_or(desc.size).width as u32, desc.min_size.unwrap_or(desc.size).height as u32)));
            window.set_max_size(Some((desc.max_size.unwrap_or(desc.size).width as u32, desc.max_size.unwrap_or(desc.size).height as u32)));

            window.commit();

            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: wgpu::Backends::VULKAN,
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
                Err(e) => panic_and_print("Adapter creation failed".to_string()),
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
                Err(e) => panic_and_print("device creation failed".to_string()),
            };

            let compositor = Compositor {
                blitter: RefCell::new(None),
                custom: RefCell::new(None),
            };

            let vello_renderer = {
                let renderer = match vello::Renderer::new(
                    &device,
                    vello::RendererOptions {
                        use_cpu: false,
                        antialiasing_support: AaSupport::all(),
                        num_init_threads: None,
                        pipeline_cache: None,
                    },
                ) {
                    Ok(r) => r,
                    Err(e) => panic_and_print("vello creation failed".to_string()),
                };

                Rc::new(RefCell::new(renderer))
            };

            let gpu_ctx = Rc::new(GpuCtx {
                instance,
                adapter,
                device,
                queue,
                compositor,
            });
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
            let raw_display_handle = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(NonNull::new(conn.backend().display_ptr() as *mut _).unwrap()));
            let raw_window_handle = RawWindowHandle::Wayland(WaylandWindowHandle::new(NonNull::new(window.wl_surface().id().as_ptr() as *mut _).unwrap()));
            let wgpu_surface: wgpu::Surface<'static> = unsafe {
                gpu_ctx.instance
                    .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                        raw_display_handle,
                        raw_window_handle,
                    })
                    .unwrap()
            };
            let mut simple_window = RosinWaylandWindow {
                registry_state: RegistryState::new(&globals),
                seat_state: SeatState::new(&globals, &qh),
                output_state: OutputState::new(&globals, &qh),

                exit: false,
                width: desc.size.width as u32,
                height: desc.size.height as u32,
                window,
                gpu_ctx,
                vello_renderer,
                tex_to_render: vello_texture,
                surface: wgpu_surface
            };

            loop {
                event_queue.blocking_dispatch(&mut simple_window).unwrap();
                if simple_window.exit {
                    println!("exiting example");
                    break;
                }
            }
        }

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
