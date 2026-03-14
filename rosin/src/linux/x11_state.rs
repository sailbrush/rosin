use crate::{
    gpu::GpuCtx,
    linux::util::{
        convert_keyboard_event_pressed_x11, convert_keyboard_event_released_x11, convert_mouse_button_pressed_x11, convert_mouse_button_released_x11, convert_mouse_motion_x11,
    },
};
use rosin_core::{
    peniko,
    prelude::Viewport,
    vello,
    wgpu::{self, TextureViewDescriptor, util::TextureBlitter},
};
use std::ops::Index;
use smithay_client_toolkit::reexports::calloop::Result;
use std::cell::RefCell;
use std::rc::Rc;
use x11rb::{
    connection::Connection,
    errors::ReplyError,
    protocol::{
        render::{self, ConnectionExt, PictType},
        xproto::Visualid,
    },
};

// referenced winit for atoms
macro_rules! atom_manager {
    ($($name:ident $(:$lit:literal)?),*) => {
        x11rb::atom_manager! {
            /// The atoms used by `winit`
            pub AtomCollection: AtomsCookie {
                $($name $(:$lit)?,)*
            }
        }

        /// Indices into the `Atoms` struct.
        #[derive(Copy, Clone, Debug)]
        #[allow(non_camel_case_types)]
        pub enum AtomName {
            $($name,)*
        }

        impl AtomName {
            pub(crate) fn atom_from(
                self,
                atoms: &AtomCollection
            ) -> &x11rb::protocol::xproto::Atom {
                match self {
                    $(AtomName::$name => &atoms.$name,)*
                }
            }
        }
    };
}
atom_manager! {
    // General Use Atoms
    CARD32,
    UTF8_STRING,
    WM_CHANGE_STATE,
    WM_CLIENT_MACHINE,
    WM_DELETE_WINDOW,
    WM_PROTOCOLS,
    WM_STATE,
    XIM_SERVERS,

    // Assorted ICCCM Atoms
    _NET_WM_ICON,
    _NET_WM_MOVERESIZE,
    _NET_WM_NAME,
    _NET_WM_PID,
    _NET_WM_PING,
    _NET_WM_SYNC_REQUEST,
    _NET_WM_SYNC_REQUEST_COUNTER,
    _NET_WM_STATE,
    _NET_WM_STATE_ABOVE,
    _NET_WM_STATE_BELOW,
    _NET_WM_STATE_FULLSCREEN,
    _NET_WM_STATE_HIDDEN,
    _NET_WM_STATE_MAXIMIZED_HORZ,
    _NET_WM_STATE_MAXIMIZED_VERT,
    _NET_WM_WINDOW_TYPE,

    // Drag-N-Drop Atoms
    XdndAware,
    XdndEnter,
    XdndLeave,
    XdndDrop,
    XdndPosition,
    XdndStatus,
    XdndActionPrivate,
    XdndSelection,
    XdndFinished,
    XdndTypeList,
    TextUriList: b"text/uri-list",
    None: b"None",

    // Stylus Atoms
    ABS_X: b"Abs X",
    ABS_Y: b"Abs Y",
    ABS_PRESSURE: b"Abs Pressure",
    ABS_TILT_X: b"Abs Tilt X",
    ABS_TILT_Y: b"Abs Tilt Y"

}
impl Index<AtomName> for AtomCollection {
    type Output = x11rb::protocol::xproto::Atom;

    fn index(&self, index: AtomName) -> &Self::Output {
        index.atom_from(self)
    }
}
pub(crate) fn choose_visual(conn: &impl Connection, screen_num: usize) -> core::result::Result<(u8, Visualid), ReplyError> {
    let depth = 32;
    let screen = &conn.setup().roots[screen_num];

    // Try to use XRender to find a visual with alpha support
    let has_render = conn.extension_information(render::X11_EXTENSION_NAME).unwrap().is_some();
    if has_render {
        let formats = conn.render_query_pict_formats().unwrap().reply().unwrap();
        // Find the ARGB32 format that must be supported.
        let format = formats
            .formats
            .iter()
            .filter(|info| (info.type_, info.depth) == (PictType::DIRECT, depth))
            .filter(|info| {
                let d = info.direct;
                (d.red_mask, d.green_mask, d.blue_mask, d.alpha_mask) == (0xff, 0xff, 0xff, 0xff)
            })
            .find(|info| {
                let d = info.direct;
                (d.red_shift, d.green_shift, d.blue_shift, d.alpha_shift) == (16, 8, 0, 24)
            });
        if let Some(format) = format {
            // Now we need to find the visual that corresponds to this format
            if let Some(visual) = formats.screens[screen_num]
                .depths
                .iter()
                .flat_map(|d| &d.visuals)
                .find(|v| v.format == format.id)
            {
                return Ok((format.depth, visual.visual));
            }
        }
    }
    Ok((screen.root_depth, screen.root_visual))
}

pub(crate) struct RosinX11Window<S: Sync + 'static> {
    pub(crate) atoms: AtomCollection,
    pub(crate) gpu_ctx: Rc<GpuCtx>,
    pub(crate) vello_renderer: Rc<RefCell<vello::Renderer>>,
    pub(crate) tex_to_render: wgpu::Texture,
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) viewport: Viewport<S, crate::handle::WindowHandle>,
    pub(crate) app_state: Rc<RefCell<S>>,
    pub(crate) window_handle: crate::handle::WindowHandle,
}

impl<S: Sync + 'static> RosinX11Window<S> {
    pub fn draw(&mut self) {
        let adapter = &self.gpu_ctx.adapter;
        let surface = &self.surface;
        let device = &self.gpu_ctx.device;
        let queue = &self.gpu_ctx.queue;

        let cap = surface.get_capabilities(&adapter);

        let surface_texture = surface.get_current_texture().expect("failed to acquire next swapchain texture");

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
        self.viewport.dispatch_event_queue(&mut state, &self.window_handle);

        let params = vello::RenderParams {
            base_color: peniko::Color::BLACK,
            width: self.tex_to_render.width(),
            height: self.tex_to_render.height(),
            antialiasing_method: vello::AaConfig::Msaa16,
        };


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
    pub fn configure(&mut self, width: u32, height: u32) {
        let adapter = &self.gpu_ctx.adapter;
        let surface = &self.surface;
        let cap = surface.get_capabilities(&adapter);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: cap.formats[0],
            view_formats: vec![cap.formats[0]],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width,
            height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        surface.configure(&self.gpu_ctx.device, &surface_config);
    }
    pub fn run_loop(&mut self, conn: &impl Connection) -> Result<()> {
        loop {
            use x11rb::protocol::Event;
            let _ = conn.flush();
            let event = conn.wait_for_event().unwrap();
            let mut redraw = false;
            let mut event_option = Some(event);
            while let Some(ref event) = event_option {
                match event {
                    Event::Expose(_) => {}
                    Event::ConfigureNotify(event) => {
                        self.configure(event.width as u32, event.height as u32);
                        redraw = true;
                    }
                    Event::ClientMessage(event) => {
                        let data = event.data.as_data32();
                        if event.format == 32 && event.window == self.window_handle.0.x11_handle.unwrap() && data[0] == self.atoms.WM_DELETE_WINDOW {
                            println!("Window was asked to close");
                            return Ok(());
                        }
                    }
                    Event::KeyPress(event) => {
                        let e = convert_keyboard_event_pressed_x11(event);
                        self.viewport.queue_keyboard_event(&e);
                        redraw = true;
                    }
                    Event::KeyRelease(event) => {
                        let e = convert_keyboard_event_released_x11(event);
                        self.viewport.queue_keyboard_event(&e);
                        redraw = true;
                    }
                    Event::ButtonPress(event) => {
                        let e = convert_mouse_button_pressed_x11(event);
                        self.viewport.queue_pointer_down_event(&e);
                        redraw = true;
                    }
                    Event::ButtonRelease(event) => {
                        let e = convert_mouse_button_released_x11(event);
                        self.viewport.queue_pointer_up_event(&e);
                        redraw = true;
                    }
                    Event::MotionNotify(event) => {
                        self.viewport.queue_pointer_move_event(&convert_mouse_motion_x11(event));
                    }
                    Event::Error(_) => println!("Got an unexpected error"),
                    _ => println!("{event:?})"),
                }
                event_option = conn.poll_for_event().unwrap();
            }
            if redraw {
                self.draw();
            }
        }
    }
}
