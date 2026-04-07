use crate::gpu::GpuCtx;
use crate::kurbo::Point;
use crate::kurbo::Vec2;
use crate::linux::csd_frame::frame::FallbackFrame;
use crate::linux::util::convert_wayland_key;
use crate::linux::util::linux_mouse_btn_convert;
use crate::linux::util::valid_char;
use crate::peniko;
use crate::prelude::OverlayPipeline;
use crate::prelude::WgpuCtx;
use crate::prelude::WgpuFn;
use crate::wgpu::TextureViewDescriptor;
use rosin_core::prelude::PointerButton;
use rosin_core::prelude::PointerEvent;
use rosin_core::viewport::Viewport;
use rosin_core::{
    vello::{self},
    wgpu,
};
use std::borrow::Cow;
use std::cell::RefCell;
use std::num::NonZero;
use std::rc::Rc;
use std::time::Duration;
use wayland_backend::client::ObjectId;
use wayland_client::Dispatch;
use wayland_client::Proxy;
use wayland_client::WEnum;
use wayland_client::globals::GlobalListContents;
use wayland_client::protocol::wl_compositor;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_keyboard;
use wayland_client::protocol::wl_pointer;
use wayland_client::protocol::wl_pointer::WlPointer;
use wayland_client::protocol::wl_registry;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_seat;
use wayland_client::protocol::wl_shm;
use wayland_client::protocol::wl_subcompositor;
use wayland_client::protocol::wl_subsurface;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, EventQueue, QueueHandle, protocol::wl_surface};
use wayland_csd_frame::DecorationsFrame;
use wayland_csd_frame::FrameAction;
use wayland_csd_frame::FrameClick;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1;
use wayland_protocols::xdg::shell::client::xdg_surface;
use wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel;
use wayland_protocols::xdg::shell::client::xdg_wm_base;
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;
use crate::linux::util::csd_resize_to_wayland;
pub(crate) const SRGB_SHADER: &str = r#"
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0)
    );
    return vec4<f32>(positions[vertex_index], 0.0, 1.0);
}

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

fn gamma(color: vec4<f32>) -> vec4<f32> {
    return color;
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dim = textureDimensions(t_diffuse);
    let uv = pos.xy / vec2<f32>(f32(dim.x), f32(dim.y));
    return gamma(textureSample(t_diffuse, s_diffuse, uv));
}
"#;

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
    pub(crate) last_mouse_pos: Vec2,
    pub(crate) wgpufn: Option<WgpuFn<S>>,
    pub(crate) pressed_modifiers: u32,
    pub(crate) fallback_frame: Option<FallbackFrame<RosinWaylandState<S>>>,
    pub(crate) last_surface_id: ObjectId,
    pub(crate) seat: Option<wl_seat::WlSeat>,
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

        let params = vello::RenderParams {
            base_color: peniko::Color::TRANSPARENT,
            width: self.width,
            height: self.height,
            antialiasing_method: vello::AaConfig::Msaa16,
        };

        self.viewport.dispatch_event_queue(&mut self.app_state.borrow_mut(), &self.window_handle);

        if let Some(wgpufn) = self.wgpufn {
            let mut encoder = self.gpu_ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Callback Encoder"),
            });

            let mut render_ctx = WgpuCtx {
                device: &self.gpu_ctx.device,
                queue: &self.gpu_ctx.queue,
                target: &swapchain_view,
                target_format: surface_texture.texture.format(),
                encoder: &mut encoder,
            };

            (wgpufn.func)(&mut self.app_state.borrow_mut(), &mut render_ctx);

            self.gpu_ctx.queue.submit(Some(encoder.finish()));
        }

        let scene = self.viewport.frame(&self.app_state.borrow_mut());
        self.vello_renderer
            .borrow_mut()
            .render_to_texture(device, queue, scene, &texture_view, &params)
            .expect("TODO: panic message");

        if self.wgpufn.is_some() {
            let mut compositor = self.gpu_ctx.compositor.custom.borrow_mut();
            let compositor = compositor.get_or_insert_with(|| {
                let shader = self.gpu_ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Rosin Compositor Shader"),
                    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(crate::gpu::COMPOSITE_SHADER)),
                });

                let layout = self.gpu_ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Rosin Compositor Layout"),
                    entries: [
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ]
                    .as_slice(),
                });

                let pipeline_layout = self.gpu_ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Rosin Compositor Pipeline Layout"),
                    bind_group_layouts: [&layout].as_slice(),
                    immediate_size: 0,
                });

                let pipeline = self.gpu_ctx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Rosin Compositor Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        compilation_options: Default::default(),
                        buffers: [].as_slice(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        compilation_options: Default::default(),
                        targets: [Some(wgpu::ColorTargetState {
                            format: cap.formats[0],
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })]
                        .as_slice(),
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview_mask: None,
                    cache: None,
                });

                let sampler = self.gpu_ctx.device.create_sampler(&wgpu::SamplerDescriptor {
                    label: Some("Rosin Compositor Sampler"),
                    address_mode_u: wgpu::AddressMode::ClampToEdge,
                    address_mode_v: wgpu::AddressMode::ClampToEdge,
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Linear,
                    ..Default::default()
                });

                OverlayPipeline { pipeline, layout, sampler }
            });

            // Queue gpu commands
            let bind_group = self.gpu_ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Compositor Bind Group"),
                layout: &compositor.layout,
                entries: [
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&compositor.sampler),
                    },
                ]
                .as_slice(),
            });

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Rosin UI Composite Pass"),
                color_attachments: [Some(wgpu::RenderPassColorAttachment {
                    view: &swapchain_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })]
                .as_slice(),
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&compositor.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        } else {
            let mut compositor = self.gpu_ctx.compositor.blitter.borrow_mut();
            let _compositor = compositor.get_or_insert_with(|| wgpu::util::TextureBlitter::new(&self.gpu_ctx.device, cap.formats[0]));

            // Queue gpu commands
            let shader = self.gpu_ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("gamma"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SRGB_SHADER)),
            });

            let layout = self.gpu_ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Rosin gamma Layout"),
                entries: [
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ]
                .as_slice(),
            });

            let pipeline_layout = self.gpu_ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Rosin gamma Pipeline Layout"),
                bind_group_layouts: [&layout].as_slice(),
                immediate_size: 0,
            });
            let pipeline = self.gpu_ctx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("gamma pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: [].as_slice(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: [Some(wgpu::ColorTargetState {
                        format: cap.formats[0],
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })]
                    .as_slice(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

            let sampler = self.gpu_ctx.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("gamma Sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });

            let bind_group = self.gpu_ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("gamma Bind Group"),
                layout: &layout,
                entries: [
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ]
                .as_slice(),
            });

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Rosin UI gamma Pass"),
                color_attachments: [Some(wgpu::RenderPassColorAttachment {
                    view: &swapchain_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })]
                .as_slice(),
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit(Some(encoder.finish()));
        surface_texture.present();
        if self.fallback_frame.is_some() {
            self.fallback_frame.as_mut().unwrap().draw();
        }
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
        if self.fallback_frame.is_some() {
            self.fallback_frame
                .as_mut()
                .unwrap()
                .resize(NonZero::new(self.width).unwrap(), NonZero::new(self.height).unwrap());
        }
    }
    pub fn run_loop(&mut self, mut event_queue: EventQueue<RosinWaylandState<S>>) -> Result<(), ()> {
        loop {
            event_queue.dispatch_pending(self).unwrap();
            self.draw();
            if self.exit {
                return Ok(());
            }
        }
        Ok(())
    }
}

impl<S: Sync + 'static> Dispatch<wl_registry::WlRegistry, GlobalListContents> for RosinWaylandState<S> {
    fn event(
        _: &mut RosinWaylandState<S>,
        _: &WlRegistry,
        _: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &wayland_client::Connection,
        _: &QueueHandle<RosinWaylandState<S>>,
    ) {
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
        data: &mut RosinWaylandState<S>,
        _: &XdgToplevel,
        event: <XdgToplevel as Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &QueueHandle<RosinWaylandState<S>>,
    ) {
        if let xdg_toplevel::Event::Close = event {
            data.exit = true;
        }
        if let xdg_toplevel::Event::Configure { width, height, states: _ } = event
            && width != 0
            && height != 0
        {
            let w = if data.fallback_frame.is_some() && width as u32 != data.width {
                data.fallback_frame
                    .as_mut()
                    .unwrap()
                    .subtract_borders(NonZero::new(width as u32).unwrap(), NonZero::new(height as u32).unwrap())
                    .0
                    .unwrap()
                    .into()
            } else {
                width as u32
            };

            let h = if data.fallback_frame.is_some() && height as u32 != data.height {
                data.fallback_frame
                    .as_mut()
                    .unwrap()
                    .subtract_borders(NonZero::new(width as u32).unwrap(), NonZero::new(height as u32).unwrap())
                    .1
                    .unwrap()
                    .into()
            } else {
                height as u32
            };
            data.width = w;
            data.height = h;
        }
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
impl<S: Sync + 'static> Dispatch<wl_seat::WlSeat, ()> for RosinWaylandState<S> {
    fn event(data: &mut RosinWaylandState<S>, seat: &wl_seat::WlSeat, event: wl_seat::Event, _: &(), _: &Connection, qh: &QueueHandle<RosinWaylandState<S>>) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(capabilities),
        } = event
        {
            data.seat = Some(seat.clone());
            if capabilities.contains(wl_seat::Capability::Keyboard) {
                seat.get_keyboard(qh, ());
            }
            if capabilities.contains(wl_seat::Capability::Pointer) {
                seat.get_pointer(qh, ());
            }
        }
    }
}
use crate::linux::util::kb_event_to_str;
impl<S: Sync + 'static> Dispatch<wl_keyboard::WlKeyboard, ()> for RosinWaylandState<S> {
    fn event(
        s: &mut RosinWaylandState<S>,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<RosinWaylandState<S>>,
    ) {
        match event {
            wl_keyboard::Event::Keymap { format: _, fd: _, size: _ } => {}
            wl_keyboard::Event::Key {
                serial: _,
                time: _,
                key,
                state,
            } => {
                let mut input_handle = s.window_handle.0.input_handler.write();

                let e = convert_wayland_key(key, state, s.pressed_modifiers);

                if let Some(handler) = input_handle.handler.as_mut() {
                    let text = kb_event_to_str(&e);
                    if text.chars().last().is_some() && valid_char(text.chars().last().unwrap()) && e.state == rosin_core::keyboard_types::KeyState::Down {
                        let text_len = text.len();

                        // Determine the range in the document to overwrite.
                        let range: std::ops::Range<usize> = handler.composition_range().unwrap_or_else(|| handler.selection());

                        let start = range.start;
                        handler.replace_range(range, &text);

                        // Update selection to end of inserted text to prevent backward typing
                        let new_cursor_pos = start + text_len;
                        handler.set_selection(new_cursor_pos..new_cursor_pos);

                        // Text is committed, so there is no longer "marked" text.
                        handler.set_composition_range(None);
                        s.viewport.queue_change_event(input_handle.id.expect("panic"));
                    } else {
                        if text.chars().last().is_some() && text.ends_with('\u{8}') && e.state == rosin_core::keyboard_types::KeyState::Down {
                            // Determine the range in the document to overwrite.
                            let range = handler.composition_range().unwrap_or_else(|| handler.selection());
                            if range.start != 0 {
                                let start = range.start - 1;
                                handler.handle_action(crate::ime::Action::Delete(crate::ime::Movement::Grapheme(crate::ime::HorizontalDirection::Left)));

                                // Update selection to end of inserted text to prevent backward typing
                                let new_cursor_pos = start;
                                handler.set_selection(new_cursor_pos..new_cursor_pos);

                                // Text is committed, so there is no longer "marked" text.
                                handler.set_composition_range(None);
                                s.viewport.queue_change_event(input_handle.id.expect("panic"));
                            }
                        }
                    }
                }
            }
            wl_keyboard::Event::Modifiers {
                serial: _,
                mods_depressed,
                mods_latched,
                mods_locked,
                group: _,
            } => {
                s.pressed_modifiers = mods_depressed | mods_latched | mods_locked;
            }
            _ => {
                println!("{:?}", event);
            }
        };
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
impl<S: Sync + 'static> Dispatch<WlPointer, ()> for RosinWaylandState<S> {
    fn event(
        data: &mut RosinWaylandState<S>,
        _pointer: &WlPointer,
        event: wl_pointer::Event,
        _udata: &(),
        _conn: &Connection,
        _qh: &QueueHandle<RosinWaylandState<S>>,
    ) {
        match event {
            wl_pointer::Event::Enter {
                surface,
                surface_x,
                surface_y,
                serial: _,
            } => {
                let _pe = PointerEvent { ..Default::default() };
                data.last_surface_id = surface.id();
                if data.fallback_frame.is_some() {
                    data.fallback_frame
                        .as_mut()
                        .unwrap()
                        .click_point_moved(Duration::new(0, 0), &data.last_surface_id, surface_x, surface_y);
                }
                //data.viewport.queue_pointer_move_event(&pe);
            }
            wl_pointer::Event::Leave { surface: _, serial: _ } => {}
            wl_pointer::Event::Motion { time: _, surface_x, surface_y } => {
                data.last_mouse_pos = Vec2::new(surface_x, surface_y);
                let pe = PointerEvent {
                    viewport_pos: Point::new(data.last_mouse_pos.x, data.last_mouse_pos.y),
                    ..Default::default()
                };

                if data.fallback_frame.is_some() {
                    data.fallback_frame
                        .as_mut()
                        .unwrap()
                        .click_point_moved(Duration::new(0, 0), &data.last_surface_id, surface_x, surface_y);
                }
                data.viewport.queue_pointer_move_event(&pe);
            }
            wl_pointer::Event::Button {
                time: _,
                button,
                state,
                serial,
            } => {
                let consumed = false;
                let pe = PointerEvent {
                    viewport_pos: Point::new(data.last_mouse_pos.x, data.last_mouse_pos.y),
                    button: linux_mouse_btn_convert(button as u16),
                    ..Default::default()
                };

                if data.fallback_frame.is_some() {
                    let action = data.fallback_frame.as_mut().unwrap().on_click(
                        Duration::new(0, 0),
                        if pe.button == PointerButton::Primary {
                            FrameClick::Normal
                        } else {
                            FrameClick::Alternate
                        },
                        state == WEnum::Value(wl_pointer::ButtonState::Pressed),
                    );
                    match action {
                        Some(FrameAction::Close) => {
                            data.exit = true;
                        }
                        Some(FrameAction::Maximize) => {
                            data.window_handle.maximize();
                        }
                        Some(FrameAction::UnMaximize) => {
                            data.window_handle.restore();
                        }
                        Some(FrameAction::Minimize) => {
                            data.window_handle.minimize();
                        }
                        Some(FrameAction::Resize(edge)) => {
                            data.window_handle.0.wayland_handle.as_mut().unwrap().xdg_toplevel.resize(
                                data.seat.as_ref().unwrap(),
                                serial,
                                csd_resize_to_wayland(edge)
                            );
                        }
                        Some(FrameAction::Move) => {
                            data.window_handle.0.wayland_handle.as_mut().unwrap().xdg_toplevel._move(
                                data.seat.as_ref().unwrap(),
                                serial
                            );
                        }
                        _ => {
                            println!("{:?}", action);
                        }
                    }
                }
                match state {
                    WEnum::Value(wl_pointer::ButtonState::Pressed) => {
                        if !consumed {
                            data.viewport.queue_pointer_down_event(&pe);
                        }
                    }
                    WEnum::Value(wl_pointer::ButtonState::Released) => {
                        if !consumed {
                            data.viewport.queue_pointer_up_event(&pe);
                        }
                    }
                    WEnum::Unknown(_unknown) => {}
                    _ => unreachable!(),
                }
            }
            wl_pointer::Event::Axis { time: _, axis, value } => match axis {
                WEnum::Value(axis) => {
                    let (mut horizontal, mut vertical) = <(f64, f64)>::default();
                    match axis {
                        wl_pointer::Axis::VerticalScroll => {
                            vertical = value;
                        }
                        wl_pointer::Axis::HorizontalScroll => {
                            horizontal = value;
                        }
                        _ => unreachable!(),
                    };

                    let pe = PointerEvent {
                        wheel_delta: Vec2::new(horizontal, vertical),
                        ..Default::default()
                    };
                    data.viewport.queue_pointer_wheel_event(&pe);
                }
                WEnum::Unknown(_unknown) => {}
            },
            wl_pointer::Event::AxisSource { axis_source } => match axis_source {
                WEnum::Value(_source) => {
                    println!("{:?}", event);
                    let _pe = PointerEvent {
                        wheel_delta: Vec2::new(0.0, 0.0),
                        ..Default::default()
                    };
                }
                WEnum::Unknown(_unknown) => {}
            },
            wl_pointer::Event::Frame => {}
            _ => println!("{:?}", event),
        };
    }
}

impl<S: Sync + 'static> Dispatch<wl_subcompositor::WlSubcompositor, ()> for RosinWaylandState<S> {
    fn event(
        _: &mut RosinWaylandState<S>,
        _: &wl_subcompositor::WlSubcompositor,
        _: <wl_subcompositor::WlSubcompositor as Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &QueueHandle<RosinWaylandState<S>>,
    ) {
    }
}

impl<S: Sync + 'static> Dispatch<wl_subsurface::WlSubsurface, ()> for RosinWaylandState<S> {
    fn event(
        _: &mut RosinWaylandState<S>,
        _: &wl_subsurface::WlSubsurface,
        _: <wl_subsurface::WlSubsurface as Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &QueueHandle<RosinWaylandState<S>>,
    ) {
    }
}

impl<S: Sync + 'static> Dispatch<wl_shm::WlShm, ()> for RosinWaylandState<S> {
    fn event(
        _: &mut RosinWaylandState<S>,
        _: &wl_shm::WlShm,
        _: <wl_shm::WlShm as Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &QueueHandle<RosinWaylandState<S>>,
    ) {
    }
}
use wayland_protocols::wp::tablet::zv2::client::zwp_tablet_manager_v2;
impl<S: Sync + 'static> Dispatch<zwp_tablet_manager_v2::ZwpTabletManagerV2, ()> for RosinWaylandState<S> {
    fn event(
        _: &mut RosinWaylandState<S>,
        _: &zwp_tablet_manager_v2::ZwpTabletManagerV2,
        _: <zwp_tablet_manager_v2::ZwpTabletManagerV2 as Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &QueueHandle<RosinWaylandState<S>>,
    ) {
    }
}

use wayland_protocols::wp::tablet::zv2::client::zwp_tablet_seat_v2;
impl<S: Sync + 'static> Dispatch<zwp_tablet_seat_v2::ZwpTabletSeatV2, ()> for RosinWaylandState<S> {
    fn event(
        _: &mut RosinWaylandState<S>,
        _: &zwp_tablet_seat_v2::ZwpTabletSeatV2,
        event: <zwp_tablet_seat_v2::ZwpTabletSeatV2 as Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &QueueHandle<RosinWaylandState<S>>,
    ) {
        match event {
            zwp_tablet_seat_v2::Event::TabletAdded { id: _ } => {}
            zwp_tablet_seat_v2::Event::ToolAdded { id: _ } => {}
            zwp_tablet_seat_v2::Event::PadAdded { id: _ } => {}
            _ => {}
        };
    }
}
use crate::{desc::WindowDesc};

use std::any::Any;
use std::sync::Arc;
use wayland_client::globals::BindError;
use wayland_client::globals::GlobalList;
use wayland_protocols::wp::tablet::zv2::client::zwp_tablet_tool_v2;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::Mode;
pub struct GlobalData;
#[derive(Debug, Clone)]
pub struct WindowData();
pub struct WaylandWindow {
    pub(crate) xdg_surface: xdg_surface::XdgSurface,
    pub(crate) xdg_toplevel: xdg_toplevel::XdgToplevel,
    pub(crate) surface: wl_surface::WlSurface,
    pub(crate) xdg_decoration_manager: Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
    pub(crate) toplevel_decoration: Option<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>,
    pub(crate) shm: Option<wl_shm::WlShm>,
    pub(crate) subcompositor: Arc<wl_subcompositor::WlSubcompositor>,
    pub(crate) compositor: Arc<wl_compositor::WlCompositor>,
    pub(crate) tablet: Option<zwp_tablet_tool_v2::ZwpTabletToolV2>,
    pub(crate) conn: Option<Connection>
}

pub(crate) fn create_window_wayland<S: Any + Sync + 'static>(
    _desc: &WindowDesc<S>,
    globals: &GlobalList,
    qh: &QueueHandle<RosinWaylandState<S>>,
) -> Arc<WaylandWindow> {
    let wl_compositor: wl_compositor::WlCompositor = globals.bind(qh, 1..=6, ()).unwrap();
    let surface = wl_compositor.create_surface(qh, ());

    let xdg_wm_base: xdg_wm_base::XdgWmBase = globals.bind(qh, 1..=6, ()).unwrap();

    let seat: wl_seat::WlSeat = globals.bind(qh, 1..=6, ()).unwrap();

    let freeze = qh.freeze();

    let window = Arc::new_cyclic(|_weak| {
        let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, qh, ());
        let xdg_toplevel = xdg_surface.get_toplevel(qh, ());
        let xdg_decoration_manager: Result<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, BindError> = globals.bind(qh, 1..=1, ());
        surface.commit();
        let toplevel_decoration = {
            if let Ok(ref xdg_deco) = xdg_decoration_manager && false {
                let toplevel_decoration = xdg_deco.get_toplevel_decoration(&xdg_toplevel, qh, ());
                toplevel_decoration.set_mode(Mode::ServerSide);
                Some(toplevel_decoration)
            } else {
                None
            }
        };
        use wayland_protocols::wp::tablet::zv2::client::zwp_tablet_manager_v2;
        let tablet_manager: zwp_tablet_manager_v2::ZwpTabletManagerV2 = globals.bind(qh, 1..=2, ()).unwrap();
        let _tablet_seat = tablet_manager.get_tablet_seat(&seat, qh, ());
        WaylandWindow {
            xdg_surface,
            xdg_toplevel,
            surface,
            xdg_decoration_manager: xdg_decoration_manager.ok(),
            toplevel_decoration,
            shm: Some(globals.bind(qh, 1..=1, ()).unwrap()),
            subcompositor: Arc::new(globals.bind(qh, 1..=1, ()).unwrap()),
            compositor: Arc::new(wl_compositor),
            tablet: None,
            conn: None
        }
    });
    // Explicitly drop the queue freeze to allow the queue to resume work.
    drop(freeze);

    window
}
