use crate::gpu::GpuCtx;
use crate::peniko;
use crate::wgpu::TextureViewDescriptor;
use crate::wgpu::util::TextureBlitter;
use rosin_core::viewport::Viewport;
use rosin_core::{
    vello::{self},
    wgpu,
};
use std::cell::RefCell;
use std::rc::Rc;
use wayland_client::Dispatch;
use wayland_client::{Connection, EventQueue, QueueHandle, protocol::wl_surface};
use wayland_protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1;

pub(crate) struct RosinWaylandState<S: Sync + 'static> {
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

impl<S: Sync + 'static> RosinWaylandState<S> {
    pub fn draw(&mut self) {
        let adapter = &self.gpu_ctx.adapter;
        let surface = &self.surface;
        let device = &self.gpu_ctx.device;
        let queue = &self.gpu_ctx.queue;
        let surface_texture = surface.get_current_texture().expect("failed to acquire next swapchain texture");

        let cap = surface.get_capabilities(adapter);
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
                color_attachments: [Some(color_attachment)].as_slice(),
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        let mut state = self.app_state.borrow_mut();

        let params = vello::RenderParams {
            base_color: peniko::Color::BLACK,
            width: self.width,
            height: self.height,
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
    pub fn configure(&mut self) {
        let adapter = &self.gpu_ctx.adapter;
        let surface = &self.surface;

        let cap = surface.get_capabilities(adapter);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: cap.formats[0],
            view_formats: vec![cap.formats[0]],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.width,
            height: self.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        surface.configure(&self.gpu_ctx.device, &surface_config);
    }
    pub fn run_loop(&mut self, mut event_queue: EventQueue<RosinWaylandState<S>>) -> Result<(), ()> {
        loop {
            event_queue.blocking_dispatch(self).unwrap();
            self.draw();
            if self.exit {
                return Ok(());
            }
        }
    }
}

use wayland_client::Proxy;
use wayland_client::globals::GlobalListContents;
use wayland_client::protocol::wl_compositor;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_registry;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1;
use wayland_protocols::xdg::shell::client::xdg_surface;
use wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel;
use wayland_protocols::xdg::shell::client::xdg_wm_base;
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;
impl<S: Sync + 'static> Dispatch<wl_registry::WlRegistry, GlobalListContents> for RosinWaylandState<S> {
    fn event(
        _: &mut RosinWaylandState<S>,
        _: &WlRegistry,
        _: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &wayland_client::Connection,
        _: &QueueHandle<RosinWaylandState<S>>,
    ) {
        todo!()
    }
}

impl<S: Sync + 'static> Dispatch<wl_compositor::WlCompositor, ()> for RosinWaylandState<S> {
    fn event(
        _: &mut RosinWaylandState<S>,
        _: &WlCompositor,
        _: <WlCompositor as Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &QueueHandle<RosinWaylandState<S>>,
    ) {
        todo!()
    }
}

impl<S: Sync + 'static> Dispatch<wl_surface::WlSurface, ()> for RosinWaylandState<S> {
    fn event(
        _: &mut RosinWaylandState<S>,
        _: &WlSurface,
        _: <WlSurface as Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &QueueHandle<RosinWaylandState<S>>,
    ) {
    }
}
use wayland_protocols::xdg::shell::client::xdg_toplevel;

impl<S: Sync + 'static> Dispatch<xdg_toplevel::XdgToplevel, ()> for RosinWaylandState<S> {
    fn event(
        _: &mut RosinWaylandState<S>,
        _: &XdgToplevel,
        event: <XdgToplevel as Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &QueueHandle<RosinWaylandState<S>>,
    ) {
        if let xdg_toplevel::Event::Close = event {}
    }
}

impl<S: Sync + 'static> Dispatch<xdg_wm_base::XdgWmBase, ()> for RosinWaylandState<S> {
    fn event(
        _: &mut RosinWaylandState<S>,
        wm_base: &XdgWmBase,
        event: <XdgWmBase as Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &QueueHandle<RosinWaylandState<S>>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl<S: Sync + 'static> Dispatch<xdg_surface::XdgSurface, ()> for RosinWaylandState<S> {
    fn event(
        data: &mut RosinWaylandState<S>,
        xdg_surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _conn: &Connection,
        _qh: &QueueHandle<RosinWaylandState<S>>,
    ) {
        if let xdg_surface::Event::Configure { serial, .. } = event {
            xdg_surface.ack_configure(serial);
            data.configure();
            data.window_handle.0.wayland_handle.clone().unwrap().surface.commit();
        }
    }
}

impl<S: Sync + 'static> Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, ()> for RosinWaylandState<S> {
    fn event(
        _data: &mut RosinWaylandState<S>,
        _deco_manager: &zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
        _event: <zxdg_decoration_manager_v1::ZxdgDecorationManagerV1 as Proxy>::Event,
        _: &(),
        _conn: &Connection,
        _qh: &QueueHandle<RosinWaylandState<S>>,
    ) {
    }
}
impl<S: Sync + 'static> Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, ()> for RosinWaylandState<S> {
    fn event(
        _data: &mut RosinWaylandState<S>,
        _toplevel_deco: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        _event: <zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1 as Proxy>::Event,
        _: &(),
        _conn: &Connection,
        _qh: &QueueHandle<RosinWaylandState<S>>,
    ) {
    }
}
