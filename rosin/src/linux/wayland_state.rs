use smithay_client_toolkit::{
    compositor::*, output::*, reexports::calloop::Result, registry::*, seat::*, shell::xdg::window::{Window, WindowConfigure, WindowHandler}, *
};
use wayland_client::{
    Connection, EventQueue, QueueHandle, protocol::{wl_output, wl_seat, wl_surface}
};

use crate::gpu::GpuCtx;
use crate::linux::handle::WindowHandle;
use crate::peniko;
use crate::prelude::ViewFn;
use crate::wgpu::util::TextureBlitter;
use crate::wgpu::{TextureFormat, TextureViewDescriptor};
use rosin_core::viewport::Viewport;
use rosin_core::{
    vello::{self},
    wgpu,
};
use std::cell::RefCell;
use std::mem::swap;
use std::rc::Rc;

// based on https://github.com/Smithay/client-toolkit/blob/master/examples/wgpu.rs
pub(crate) struct RosinWaylandWindow<S: Sync + 'static> {
    pub(crate) registry_state: RegistryState,
    pub(crate) seat_state: SeatState,
    pub(crate) output_state: OutputState,

    pub(crate) exit: bool,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) gpu_ctx: Rc<GpuCtx>,
    pub(crate) vello_renderer: Rc<RefCell<vello::Renderer>>,
    pub(crate) tex_to_render: wgpu::Texture,
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) viewport: Viewport<S, crate::handle::WindowHandle>,
    pub(crate) app_state: Rc<RefCell<S>>,
    pub(crate) window_handle: crate::handle::WindowHandle,
}
impl<S: Sync + 'static> CompositorHandler for RosinWaylandWindow<S> {
    fn scale_factor_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _new_factor: i32) {
        // Not needed for this example.
    }

    fn transform_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _new_transform: wl_output::Transform) {
        // Not needed for this example.
    }

    fn frame(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _time: u32) {}

    fn surface_enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _output: &wl_output::WlOutput) {
        // Not needed for this example.
    }

    fn surface_leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _output: &wl_output::WlOutput) {
        // Not needed for this example.
    }
}

impl<S: Sync + 'static> OutputHandler for RosinWaylandWindow<S> {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}

    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}

    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
}

impl<S: Sync + 'static> WindowHandler for RosinWaylandWindow<S> {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {
        self.exit = true;
    }
    // this gets called whenever something with the window changes, ex resize, minimize, maximize, etc.
    fn configure(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _window: &Window, configure: WindowConfigure, _serial: u32) {

        let (new_width, new_height) = configure.new_size;
        self.width = new_width.map_or(self.width, |v| v.get());
        self.height = new_height.map_or(self.height, |v| v.get());

        self.configure();
        self.draw();
    }
}

impl<S: Sync + 'static> SeatHandler for RosinWaylandWindow<S> {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat, _capability: Capability) {}

    fn remove_capability(&mut self, _conn: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat, _capability: Capability) {}

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

delegate_compositor!(@<S: Sync + 'static> RosinWaylandWindow<S>);
delegate_output!(@<S: Sync + 'static> RosinWaylandWindow<S>);

delegate_seat!(@<S: Sync + 'static> RosinWaylandWindow<S>);

delegate_xdg_shell!(@<S: Sync + 'static> RosinWaylandWindow<S>);
delegate_xdg_window!(@<S: Sync + 'static> RosinWaylandWindow<S>);

delegate_registry!(@<S: Sync + 'static> RosinWaylandWindow<S>);

impl<S: Sync + 'static> ProvidesRegistryState for RosinWaylandWindow<S> {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

impl<S: Sync + 'static> RosinWaylandWindow<S> {
    pub fn draw(&mut self) {
        let adapter = &self.gpu_ctx.adapter;
        let surface = &self.surface;
        let device = &self.gpu_ctx.device;
        let queue = &self.gpu_ctx.queue;
        let surface_texture = surface.get_current_texture().expect("failed to acquire next swapchain texture");

        let cap = surface.get_capabilities(&adapter);
        let texture_view = self.tex_to_render.create_view(&TextureViewDescriptor::default());

        let mut swap_tex_desc = TextureViewDescriptor::default();
        swap_tex_desc.format = Some(cap.formats[0]);
        let swapchain_view = surface_texture.texture.create_view(&swap_tex_desc);

        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let color_attachment = wgpu::RenderPassColorAttachment {
                view: &swapchain_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            };

            let _renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(color_attachment)].as_slice(),
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        let mut state = self.app_state.borrow_mut();

        let params = vello::RenderParams {
            base_color: peniko::Color::BLACK,
            width: self.width as u32,
            height: self.height as u32,
            antialiasing_method: vello::AaConfig::Msaa16,
        };

        self.viewport.dispatch_event_queue(&mut state, &self.window_handle);

        let scene = self.viewport.frame(&state);
        self.vello_renderer
            .borrow_mut()
            .render_to_texture(device, queue, scene, &texture_view, &params)
            .expect("TODO: panic message");

        let blitter = TextureBlitter::new(&self.gpu_ctx.device, cap.formats[0]);

        blitter.copy(&self.gpu_ctx.device, &mut encoder, &texture_view, &swapchain_view);

        queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }
    pub fn configure(&mut self){

        let adapter = &self.gpu_ctx.adapter;
        let surface = &self.surface;

        let cap = surface.get_capabilities(&adapter);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: cap.formats[0],
            view_formats: vec![cap.formats[0]],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.width,
            height: self.height,
            desired_maximum_frame_latency: 2,
            // Wayland is inherently a mailbox system.
            present_mode: wgpu::PresentMode::Mailbox,
        };
        surface.configure(&self.gpu_ctx.device, &surface_config);
    }
    pub fn run_loop(&mut self, mut event_queue: EventQueue<RosinWaylandWindow<S>>) -> Result<()>{
        loop {
            event_queue.blocking_dispatch(self).unwrap();
            if self.exit {
                return Ok(());
            }
        }
    }
}
