use crate::linux::util;
use crate::linux::{
    create_window::create_window_x11,
    x11::{AtomCollection, choose_visual},
};
use crate::prelude::*;
use pollster::FutureExt;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle};
use rosin_core::vello::{self, AaSupport};
use rosin_core::wgpu;
use rosin_core::wgpu::ExperimentalFeatures;
use std::cell::RefCell;
use std::num::NonZero;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::OnceLock;
use wayland_client::Connection as WaylandConn;
use x11rb::connection::Connection as X11Conn;
use x11rb::xcb_ffi::XCBConnection;

use raw_window_handle::{XcbDisplayHandle, XcbWindowHandle};
use rosin_core::prelude::Viewport;

use crate::linux::x11::RosinX11Window;
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
        if way_conn.is_ok() && false {
            use smithay_client_toolkit::{output::OutputState, registry::RegistryState, seat::SeatState, shell::WaylandSurface};
            use wayland_client::Proxy;

            use crate::linux::{create_window::create_window_wayland, wayland::RosinWaylandWindow};

            let conn = way_conn.unwrap();
            let (globals, mut event_queue) = wayland_client::globals::registry_queue_init(&conn).unwrap();
            let qh = event_queue.handle();
            let desc = self.windows[0].clone();
            let window = create_window_wayland(&desc, &globals, &qh);
            let raw_display_handle = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(NonNull::new(conn.backend().display_ptr() as *mut _).unwrap()));
            let raw_window_handle = RawWindowHandle::Wayland(WaylandWindowHandle::new(NonNull::new(window.wl_surface().id().as_ptr() as *mut _).unwrap()));
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
                Err(e) => util::panic_and_print("GPU initialization failed".to_string()),
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
                Err(e) => util::panic_and_print("GPU initialization failed".to_string()),
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

            let wh = crate::prelude::WindowHandle {
                0: crate::linux::handle::WindowHandle {
                    wayland_handle: Some(window),
                    x11_handle: None,
                },
            };
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
                    Err(e) => util::panic_and_print("GPU initialization failed".to_string()),
                };

                Rc::new(RefCell::new(renderer))
            };
            let mut window: RosinWaylandWindow<S> = RosinWaylandWindow {
                registry_state: RegistryState::new(&globals),
                seat_state: SeatState::new(&globals, &qh),
                output_state: OutputState::new(&globals, &qh),

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
            };
            let _ = window.run_loop(event_queue);
        } else {
            let desc = self.windows[0].clone();
            let (conn, screen_num) = XCBConnection::connect(None).unwrap();
            let screen = &conn.setup().roots[screen_num];
            let atoms = AtomCollection::new(&conn).unwrap().reply().unwrap();
            let (depth, visualid) = choose_visual(&conn, screen_num).unwrap();

            let window = create_window_x11(&desc, &conn, screen, &atoms, depth, visualid).unwrap();

            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: self.wgpu_config.backends,
                ..Default::default()
            });
            let xcbWinHandle = XcbWindowHandle::new(NonZero::new(window).expect("error"));
            let raw_win_handle = RawWindowHandle::Xcb(xcbWinHandle);
            let xcbDispHandle = XcbDisplayHandle::new(NonNull::new(conn.get_raw_xcb_connection()), screen_num as i32);
            let raw_disp_handle = RawDisplayHandle::Xcb(xcbDispHandle);

            let wgpu_surface: wgpu::Surface<'static> = unsafe {
                instance
                    .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                        raw_window_handle: raw_win_handle,
                        raw_display_handle: raw_disp_handle,
                    })
                    .unwrap()
            };

            let adapter = match instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: self.wgpu_config.power_preference,
                    force_fallback_adapter: false,
                    compatible_surface: None,
                })
                .block_on()
            {
                Ok(adapter) => adapter,
                Err(e) => util::panic_and_print("GPU initialization failed".to_string()),
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
                Err(e) => util::panic_and_print("GPU initialization failed".to_string()),
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
                    Err(e) => util::panic_and_print("GPU initialization failed".to_string()),
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
            let viewport: Viewport<S, crate::prelude::WindowHandle> =
                Viewport::new(desc.viewfn.func, desc.size, rosin_core::kurbo::Vec2 { x: 1.0f64, y: 1.0f64 }, _translation_map);

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
            let wh = crate::prelude::WindowHandle {
                0: crate::linux::handle::WindowHandle {
                    wayland_handle: None,
                    x11_handle: Some(window),
                },
            };
            let mut x11Window = RosinX11Window {
                app_state: self.state.unwrap(),
                gpu_ctx,
                vello_renderer,
                surface: wgpu_surface,
                tex_to_render: vello_texture,
                viewport,
                window_handle: wh,
                atoms,
                desc,
            };

            x11Window.configure();
            x11Window.draw();
            let _ = x11Window.run_loop(&conn);
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
