use std::{
    any::Any,
    borrow::Cow,
    cell::{Cell, RefCell},
    ffi::c_void,
    rc::Rc,
    time::Instant,
};

use accesskit_macos::SubclassingAdapter;
use objc2::{
    AnyThread, ClassType, DeclaredClass, MainThreadOnly, define_class, msg_send,
    rc::{Allocated, Retained},
    runtime::{AnyObject, Bool, ProtocolObject, Sel},
    sel,
};
use objc2_app_kit::{
    NSApp, NSBackingStoreType, NSEvent, NSEventModifierFlags, NSMenu, NSMenuItem, NSTextInputClient, NSTrackingArea, NSTrackingAreaOptions, NSView,
    NSViewLayerContentsRedrawPolicy, NSWindow, NSWindowButton, NSWindowDelegate, NSWindowStyleMask,
};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSAttributedString, NSNotFound, NSNotification, NSObjectNSThreadPerformAdditions, NSObjectProtocol, NSPoint, NSRange, NSRect,
    NSRunLoop, NSRunLoopCommonModes, NSSize, NSString, NSTimer,
};
use objc2_quartz_core::{CADisplayLink, CALayer, CALayerDelegate, CAMetalLayer};

use rosin_core::viewport::*;

use crate::{
    accesskit::{self, ActionHandler, ActionRequest, NodeId as AxNodeId, Role, TreeId, TreeUpdate},
    gpu::OverlayPipeline,
    kurbo::Vec2,
    log::error,
    mac::util,
    peniko,
    prelude::*,
    vello, wgpu,
};

pub(crate) fn create_window<S: Sync + 'static>(
    mtm: MainThreadMarker,
    desc: &WindowDesc<S>,
    state: Rc<RefCell<S>>,
    translation_map: TranslationMap,
    gpu_ctx: Rc<GpuCtx>,
    vello_renderer: Rc<RefCell<vello::Renderer>>,
) {
    let mut style_mask = NSWindowStyleMask::empty();
    style_mask.insert(NSWindowStyleMask::Titled);
    if desc.close_button {
        style_mask.insert(NSWindowStyleMask::Closable);
    }
    if desc.minimize_button {
        style_mask.insert(NSWindowStyleMask::Miniaturizable);
    }
    if desc.resizeable {
        style_mask.insert(NSWindowStyleMask::Resizable);
    }

    let position = if let Some(position) = desc.position {
        NSPoint::new(position.x, position.y)
    } else {
        NSPoint::new(0.0, 0.0)
    };

    let ns_window = unsafe {
        let ns_window: Allocated<NSWindow> = mtm.alloc();
        let ns_window = NSWindow::initWithContentRect_styleMask_backing_defer(
            ns_window,
            NSRect::new(position, NSSize::new(desc.size.width, desc.size.height)),
            style_mask,
            NSBackingStoreType::Buffered,
            false,
        );

        if let Some(min_size) = &desc.min_size {
            ns_window.setContentMinSize(NSSize::new(min_size.width, min_size.height));
        }
        if let Some(max_size) = &desc.max_size {
            ns_window.setContentMaxSize(NSSize::new(max_size.width, max_size.height));
        }

        ns_window.setReleasedWhenClosed(false);
        ns_window
    };

    if let Some(title) = &desc.title {
        ns_window.setTitle(&NSString::from_str(title));
    }
    if !desc.maximize_button
        && let Some(button) = ns_window.standardWindowButton(NSWindowButton::ZoomButton)
    {
        button.setHidden(true);
    }
    if desc.position.is_none() {
        ns_window.center();
    }

    let vello_texture = gpu_ctx.device.create_texture(&wgpu::TextureDescriptor {
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
        view_formats: &[],
    });

    let scale = ns_window.backingScaleFactor();
    let ivars = ViewIvars {
        context_menu_node: Cell::new(None),
        needs_config: Cell::new(true),
        handle: RefCell::new(None),
        gpu_ctx,
        vello_renderer,
        vello_texture: RefCell::new(vello_texture),
        last_frame: Cell::new(None),
        gained_focus: Cell::new(false),
        input_handler: RefCell::new(None),
        input_handler_node: Cell::new(None),
        main_menu: RefCell::new(None),
        display_link: RefCell::new(None),
        reload_timer: RefCell::new(None),
        key_down_consumed: Cell::new(false),
        a11y_adapter: RefCell::new(None),
        viewport: RefCell::new(Box::new(ViewportContainer {
            desc: desc.clone(),
            viewport: Viewport::new(desc.viewfn.func, desc.size, Vec2::new(scale, scale), translation_map.clone()),
            app_state: state,
            surface: None,
            surface_format: None,
            wgpu_deps: Some(DependencyMap::default()),
            wgpu_deps_changed: false,
        }) as Box<dyn ViewportTrait>),
    };

    let ns_view: Retained<RosinView> = unsafe {
        let view: Allocated<RosinView> = mtm.alloc();
        let view = view.set_ivars(ivars);
        let view: Retained<RosinView> = msg_send![super(view), init];
        view.setWantsLayer(true);
        let metal_layer = CAMetalLayer::new();
        metal_layer.setDelegate(Some(ProtocolObject::from_ref(&*view)));
        view.setLayer(Some(&metal_layer));
        view.setLayerContentsRedrawPolicy(NSViewLayerContentsRedrawPolicy::DuringViewResize);
        view
    };
    ns_window.setContentView(Some(&ns_view));
    ns_window.setAcceptsMouseMovedEvents(true);

    let handle = crate::platform::handle::WindowHandle::new(mtm, ns_view.clone());
    *ns_view.ivars().handle.borrow_mut() = Some(WindowHandle(handle));

    if let Some(menu) = &desc.menu {
        ns_view.set_main_menu(Some(menu.clone()));
    }

    ns_window.setDelegate(Some(ProtocolObject::from_ref(&*ns_view)));

    let view_ptr = Retained::<RosinView>::as_ptr(&ns_view);
    // SubclassingAdapter must be installed before the view is first shown or focused.
    let adapter = unsafe {
        SubclassingAdapter::new(
            view_ptr as *mut c_void,
            RosinA11yActivationHandler {
                view: view_ptr as *mut RosinView,
            },
            RosinA11yActionHandler {
                view: view_ptr as *mut RosinView,
            },
        )
    };
    *ns_view.ivars().a11y_adapter.borrow_mut() = Some(adapter);

    unsafe {
        ns_window.performSelectorOnMainThread_withObject_waitUntilDone(sel!(makeKeyAndOrderFront:), None, false);

        // Use NSView's displayLink API to drive animations.
        // The link starts paused, and is toggled based on viewport.has_anim_nodes() after frame().
        let run_loop = NSRunLoop::mainRunLoop();
        let anim = ns_view.displayLinkWithTarget_selector(&ns_view, sel!(anim_frame:));
        anim.addToRunLoop_forMode(&run_loop, NSRunLoopCommonModes);
        anim.setPaused(true);

        if cfg!(debug_assertions) {
            let reload = NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(1.0 / 5.0, &ns_view, sel!(reload_assets), None, true);
            *ns_view.ivars().reload_timer.borrow_mut() = Some(reload);
        };

        *ns_view.ivars().display_link.borrow_mut() = Some(anim);
    }
}

struct RosinA11yActionHandler {
    view: *mut RosinView,
}

impl ActionHandler for RosinA11yActionHandler {
    fn do_action(&mut self, request: ActionRequest) {
        // SAFETY: accesskit_macos delivers actions on the main thread for the view.
        let view = unsafe { &*self.view };
        view.ivars().viewport.borrow_mut().accessibility_action_event(view, request);
    }
}

/// Builds the initial tree immediately when the accessibility system first activates.
///
/// This is what makes VoiceOver/etc get a full tree the first time, instead of waiting
/// for the first incremental update.
struct RosinA11yActivationHandler {
    view: *mut RosinView,
}

impl accesskit::ActivationHandler for RosinA11yActivationHandler {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        // SAFETY: accesskit_macos guarantees the activation handler is called on the main thread.
        let view = unsafe { &*self.view };

        // Ask the viewport for a full tree.
        match view.ivars().viewport.borrow_mut().build_accesskit_update() {
            Ok(update) => return Some(update),
            Err(e) => error!("AccessKit initial tree build failed: {e}"),
        }

        // Fallback to a minimal but valid tree.
        let root_id = AxNodeId(0);
        let root = accesskit::Node::new(Role::Window);
        Some(TreeUpdate {
            tree_id: TreeId::ROOT,
            nodes: vec![(root_id, root)],
            tree: Some(accesskit::Tree::new(root_id)),
            focus: root_id,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PointerEvent {
    Down,
    Up,
    Move,
    Leave,
    Wheel,
}

#[allow(clippy::enum_variant_names)]
pub(crate) enum WindowEvent {
    DidChangeBackingProperties,
    DidBecomeKey,
    DidResignKey,
}

pub(crate) struct ViewIvars {
    pub context_menu_node: Cell<Option<NodeId>>,
    pub display_link: RefCell<Option<Retained<CADisplayLink>>>,
    pub gained_focus: Cell<bool>,
    pub handle: RefCell<Option<WindowHandle>>,
    pub input_handler_node: Cell<Option<NodeId>>,
    pub input_handler: RefCell<Option<Box<dyn InputHandler + Send + Sync>>>,
    pub key_down_consumed: Cell<bool>,
    pub last_frame: Cell<Option<Instant>>,
    pub main_menu: RefCell<Option<Retained<NSMenu>>>,
    pub needs_config: Cell<bool>,
    pub reload_timer: RefCell<Option<Retained<NSTimer>>>,
    pub gpu_ctx: Rc<GpuCtx>,
    pub vello_renderer: Rc<RefCell<vello::Renderer>>,
    pub vello_texture: RefCell<wgpu::Texture>,
    pub viewport: RefCell<Box<dyn ViewportTrait>>,
    pub a11y_adapter: RefCell<Option<SubclassingAdapter>>,
}

// Objc objects cannot be generic, so we store the app state and viewport in a type erased object
pub(crate) struct ViewportContainer<'a, S: Sync + 'static> {
    desc: WindowDesc<S>,
    app_state: Rc<RefCell<S>>,
    surface: Option<wgpu::Surface<'a>>,
    surface_format: Option<wgpu::TextureFormat>,
    viewport: Viewport<S, WindowHandle>,
    wgpu_deps_changed: bool,
    wgpu_deps: Option<DependencyMap>,
}

pub(crate) trait ViewportTrait {
    fn create_window(&mut self, mtm: MainThreadMarker, view: &RosinView, desc: Box<dyn Any + Send + Sync>);
    fn dispatch_and_redraw(&mut self, view: &RosinView);
    fn anim_frame(&mut self, view: &RosinView);
    fn update_layer(&mut self, view: &RosinView);
    fn reload_assets(&mut self, view: &RosinView);
    fn close(&mut self, view: &RosinView) -> bool;
    fn window_event(&mut self, kind: WindowEvent, view: &RosinView);
    fn input_event(&mut self, kind: PointerEvent, ns_event: &NSEvent, view: &RosinView);
    fn command_event(&mut self, view: &RosinView, node: Option<NodeId>, command: CommandId);
    fn keyboard_event(&mut self, ns_event: &NSEvent, view: &RosinView);
    fn file_dialog_event(&mut self, view: &RosinView, node: NodeId, response: FileDialogResponse);
    fn timer_event(&mut self, node: NodeId, view: &RosinView);
    fn change_event(&mut self, node: NodeId);
    fn translation_map(&self) -> TranslationMap;
    fn build_accesskit_update(&mut self) -> Result<TreeUpdate, AccessKitUpdateError>;
    fn accessibility_action_event(&mut self, view: &RosinView, request: ActionRequest);

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    fn use_library(&mut self, lib: &libloading::Library);

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    fn serializable_window_desc(&self, view: &RosinView) -> super::hot::SerializableWindowDesc;
}

impl<'a, S: Sync + 'static> ViewportTrait for ViewportContainer<'a, S> {
    fn create_window(&mut self, mtm: MainThreadMarker, view: &RosinView, desc: Box<dyn Any + Send + Sync>) {
        let Ok(desc) = &desc.downcast::<WindowDesc<S>>() else {
            error!("Failed to create window: WindowDesc is generic over the wrong type.");
            return;
        };

        create_window(
            mtm,
            desc,
            self.app_state.clone(),
            self.viewport.get_translation_map(),
            view.ivars().gpu_ctx.clone(),
            view.ivars().vello_renderer.clone(),
        );
    }

    fn reload_assets(&mut self, view: &RosinView) {
        let _ = self.viewport.reload_stylesheets();
        let _ = self.viewport.reload_translation_map();

        if !self.viewport.is_idle() {
            view.request_redraw();
        }
    }

    fn dispatch_and_redraw(&mut self, view: &RosinView) {
        view.ivars().gained_focus.set(false);

        let handle_ref = view.ivars().handle.borrow();
        let Some(handle) = handle_ref.as_ref() else {
            return;
        };

        let mut state = self.app_state.borrow_mut();
        self.viewport.dispatch_event_queue(&mut state, handle);

        let deps_changed = self.wgpu_deps.as_ref().is_some_and(|d| d.any_changed());
        self.wgpu_deps_changed = deps_changed;

        if !self.viewport.is_idle() || deps_changed {
            view.request_redraw();
        }
    }

    fn anim_frame(&mut self, view: &RosinView) {
        let this_frame = Instant::now();
        if let Some(last_frame) = view.ivars().last_frame.get() {
            let frame_time = this_frame.duration_since(last_frame);
            self.viewport.queue_animation_events(frame_time);
        }
        view.ivars().last_frame.set(Some(this_frame));
        self.dispatch_and_redraw(view);
    }

    fn update_layer(&mut self, view: &RosinView) {
        let handle_ref = view.ivars().handle.borrow();
        let Some(handle) = handle_ref.as_ref() else {
            return;
        };

        let physical_size = handle.get_physical_size();
        if physical_size.width == 0.0 || physical_size.height == 0.0 {
            return;
        }

        let Some(ns_window) = view.window() else {
            return;
        };

        let mut properties_changed = false;

        let logical_size = handle.get_logical_size();
        if logical_size != self.viewport.get_size() {
            properties_changed = true;
            self.viewport.set_size(logical_size);
        }

        let scale = ns_window.backingScaleFactor();
        let scale = Vec2::new(scale, scale);
        if scale != self.viewport.get_scale() {
            properties_changed = true;
            self.viewport.set_scale(scale);
        }

        if self.viewport.is_idle() && !view.ivars().needs_config.get() && !self.wgpu_deps_changed {
            // There are rare situations where update_layer() will be called twice for a single refresh
            // and we don't need to update the layer in that case.
            return;
        }
        self.wgpu_deps_changed = false;

        let gpu_ctx = &view.ivars().gpu_ctx;

        let Some(layer) = view.layer().and_then(|layer| {
            if !layer.isKindOfClass(CAMetalLayer::class()) {
                return None;
            }
            // SAFETY: isKindOfClass checked above.
            unsafe { Retained::<CAMetalLayer>::from_raw(Retained::<CALayer>::into_raw(layer) as *mut CAMetalLayer) }
        }) else {
            return;
        };

        // Ensure surface exists
        if self.surface.is_none() {
            let surface = match gpu_ctx.instance.create_surface(handle.clone()) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to create wgpu surface: {e:?}");
                    return;
                }
            };

            // TODO - Wgpu v28 fixes this:
            // https://github.com/gfx-rs/wgpu/pull/8716
            // So can be fixed as soon as Vello updates
            #[allow(invalid_reference_casting)]
            unsafe {
                if let Some(hal_surface) = surface.as_hal::<wgpu::hal::api::Metal>() {
                    let raw = (&*hal_surface) as *const wgpu::hal::metal::Surface as *mut wgpu::hal::metal::Surface;
                    (*raw).present_with_transaction = true;
                }
            }

            layer.setDelegate(Some(ProtocolObject::from_ref(view)));
            self.surface = Some(surface);
        }

        let Some(surface) = self.surface.as_ref() else {
            // unreachable
            return;
        };

        if properties_changed || view.ivars().needs_config.get() {
            let capabilities = surface.get_capabilities(&gpu_ctx.adapter);
            let format = match capabilities
                .formats
                .into_iter()
                .find(|format| matches!(format, wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm))
            {
                Some(fmt) => fmt,
                None => {
                    error!("Surface doesn't support Rgba8Unorm or Bgra8Unorm");
                    return;
                }
            };

            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: physical_size.width as u32,
                height: physical_size.height as u32,
                present_mode: wgpu::PresentMode::AutoVsync,
                alpha_mode: wgpu::CompositeAlphaMode::PostMultiplied,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            surface.configure(&gpu_ctx.device, &config);
            self.surface_format = Some(format);

            let new_vello_texture = gpu_ctx.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: physical_size.width as u32,
                    height: physical_size.height as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            *view.ivars().vello_texture.borrow_mut() = new_vello_texture;
            layer.setContentsScale(ns_window.backingScaleFactor());
        }

        let surface_texture = match surface.get_current_texture() {
            Ok(tex) => tex,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost | wgpu::SurfaceError::Timeout | wgpu::SurfaceError::Other) => {
                view.ivars().needs_config.set(true);
                return;
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                error!("wgpu out of memory");
                return;
            }
        };
        view.ivars().needs_config.set(false);

        let mut state = self.app_state.borrow_mut();
        let scene = self.viewport.frame(&state);

        let begin_paint = Instant::now();

        // Run WGPU Callback
        if let Some(wgpufn) = self.desc.wgpufn {
            let target = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = gpu_ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Callback Encoder"),
            });

            let mut render_ctx = WgpuCtx {
                device: &gpu_ctx.device,
                queue: &gpu_ctx.queue,
                target: &target,
                target_format: surface_texture.texture.format(),
                encoder: &mut encoder,
            };

            let wgpu_deps = self.wgpu_deps.take().unwrap_or_default().cleared().read_scope(|| {
                (wgpufn.func)(&mut state, &mut render_ctx);
            });
            self.wgpu_deps = Some(wgpu_deps);

            gpu_ctx.queue.submit(Some(encoder.finish()));
        }

        let vello_texture_view = view.ivars().vello_texture.borrow().create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(1),
            usage: None,
        });

        let params = vello::RenderParams {
            base_color: peniko::Color::TRANSPARENT,
            width: physical_size.width as u32,
            height: physical_size.height as u32,
            antialiasing_method: vello::AaConfig::Msaa16,
        };

        if let Err(e) = view
            .ivars()
            .vello_renderer
            .borrow_mut()
            .render_to_texture(&gpu_ctx.device, &gpu_ctx.queue, scene, &vello_texture_view, &params)
        {
            error!("Failed to render to texture: {e:?}");
            view.ivars().needs_config.set(true);
            return;
        }

        let surface_texture_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(surface_texture.texture.format()),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(1),
            usage: None,
        });

        let mut encoder = gpu_ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compositing Pass"),
        });

        if self.desc.wgpufn.is_some() {
            // Init custom compositor if needed
            let mut compositor = gpu_ctx.compositor.custom.borrow_mut();
            let compositor = compositor.get_or_insert_with(|| {
                let shader = gpu_ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Rosin Compositor Shader"),
                    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(crate::gpu::COMPOSITE_SHADER)),
                });

                let layout = gpu_ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Rosin Compositor Layout"),
                    entries: &[
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
                    ],
                });

                let pipeline_layout = gpu_ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Rosin Compositor Pipeline Layout"),
                    bind_group_layouts: &[&layout],
                    push_constant_ranges: &[],
                });

                let pipeline = gpu_ctx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Rosin Compositor Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Bgra8Unorm,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });

                let sampler = gpu_ctx.device.create_sampler(&wgpu::SamplerDescriptor {
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
            let bind_group = gpu_ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Compositor Bind Group"),
                layout: &compositor.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&vello_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&compositor.sampler),
                    },
                ],
            });

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Rosin UI Composite Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&compositor.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        } else {
            // Init default compositor if needed
            let mut compositor = gpu_ctx.compositor.blitter.borrow_mut();
            let compositor = compositor.get_or_insert_with(|| wgpu::util::TextureBlitter::new(&gpu_ctx.device, wgpu::TextureFormat::Bgra8Unorm));

            // Queue gpu commands
            compositor.copy(&gpu_ctx.device, &mut encoder, &vello_texture_view, &surface_texture_view);
        }

        gpu_ctx.queue.submit(Some(encoder.finish()));

        surface_texture.present();
        self.viewport.report_paint_time(Instant::now().duration_since(begin_paint));

        // Check if the animation nodes changed after calling frame()
        // We tick animations even if they're disabled in case they get enabled by another thread.
        view.set_display_link_active(self.viewport.has_anim_nodes());

        // frame() may have queued events
        self.viewport.dispatch_event_queue(&mut state, handle);

        // Accessibility update
        let queued_events = 'qe: {
            let mut adapter_ref = view.ivars().a11y_adapter.borrow_mut();
            let Some(adapter) = adapter_ref.as_mut() else {
                break 'qe None;
            };

            adapter.update_if_active(|| match self.viewport.build_accesskit_tree(&*state) {
                Ok(update) => update,
                Err(e) => {
                    error!("AccessKit tree build failed: {e}");
                    TreeUpdate {
                        tree_id: TreeId::ROOT,
                        nodes: Vec::new(),
                        tree: None,
                        focus: AxNodeId(0),
                    }
                }
            })
        };
        if let Some(queued) = queued_events {
            queued.raise();
        }
    }

    fn close(&mut self, view: &RosinView) -> bool {
        let stop_window_close = {
            let handle_ref = view.ivars().handle.borrow();
            let Some(handle) = handle_ref.as_ref() else {
                return true;
            };

            let mut state = self.app_state.borrow_mut();

            self.viewport.queue_close_event();
            let dispatch_info = self.viewport.dispatch_event_queue(&mut state, handle).unwrap_or_default();

            dispatch_info.stop_window_close
        };

        if !stop_window_close {
            if let Some(dl) = view.ivars().display_link.borrow_mut().take() {
                dl.invalidate();
            }
            if let Some(t) = view.ivars().reload_timer.borrow_mut().take() {
                t.invalidate();
            }

            // Drop anything that holds references to the NSWindow or NSView to ensure deallocation
            *view.ivars().handle.borrow_mut() = None;
            self.surface = None;
            true
        } else {
            false
        }
    }

    fn window_event(&mut self, kind: WindowEvent, view: &RosinView) {
        match kind {
            WindowEvent::DidChangeBackingProperties => {
                view.ivars().needs_config.set(true);
            }
            WindowEvent::DidBecomeKey => {
                view.ivars().gained_focus.set(true);
                self.viewport.queue_got_focus_event();

                let mtm = unsafe { MainThreadMarker::new_unchecked() };
                let menu = view.ivars().main_menu.borrow();
                NSApp(mtm).setMainMenu(menu.as_deref());

                // Keep AccessKit's window focus state in sync.
                let queued_events = {
                    let mut adapter_ref = view.ivars().a11y_adapter.borrow_mut();
                    adapter_ref.as_mut().and_then(|adapter| adapter.update_view_focus_state(true))
                };
                if let Some(queued) = queued_events {
                    queued.raise();
                }

                self.dispatch_and_redraw(view);
            }
            WindowEvent::DidResignKey => {
                self.viewport.queue_lost_focus_event();

                // Keep AccessKit's window focus state in sync.
                let queued_events = {
                    let mut adapter_ref = view.ivars().a11y_adapter.borrow_mut();
                    adapter_ref.as_mut().and_then(|adapter| adapter.update_view_focus_state(false))
                };
                if let Some(queued) = queued_events {
                    queued.raise();
                }

                self.dispatch_and_redraw(view);
            }
        }
    }

    fn command_event(&mut self, view: &RosinView, node: Option<NodeId>, command: CommandId) {
        self.viewport.queue_command_event(node, command);
        self.dispatch_and_redraw(view);
    }

    fn input_event(&mut self, kind: PointerEvent, ns_event: &NSEvent, view: &RosinView) {
        if kind == PointerEvent::Leave {
            self.viewport.queue_pointer_leave_event();
            self.dispatch_and_redraw(view);
            return;
        }

        let mut event = util::convert_pointer_event(ns_event, view);
        event.did_focus_window = view.ivars().gained_focus.get();
        match kind {
            PointerEvent::Down => {
                self.viewport.queue_pointer_down_event(&event);
                view.invalidate_ime_rects();
            }
            PointerEvent::Up => {
                self.viewport.queue_pointer_up_event(&event);
            }
            PointerEvent::Move => {
                self.viewport.queue_pointer_move_event(&event);
            }
            PointerEvent::Wheel => {
                self.viewport.queue_pointer_wheel_event(&event);
                view.invalidate_ime_rects();
            }
            _ => {}
        }
        self.dispatch_and_redraw(view);
    }

    fn keyboard_event(&mut self, ns_event: &NSEvent, view: &RosinView) {
        let is_composing = view.ivars().input_handler.borrow().as_deref().and_then(|h| h.composition_range()).is_some();

        if let Some(event) = util::convert_keyboard_event(ns_event, is_composing) {
            self.viewport.queue_keyboard_event(&event);
        }
        self.dispatch_and_redraw(view);
    }

    fn file_dialog_event(&mut self, view: &RosinView, node: NodeId, response: FileDialogResponse) {
        self.viewport.queue_file_dialog_event(node, response);
        self.dispatch_and_redraw(view);
    }

    fn timer_event(&mut self, node: NodeId, view: &RosinView) {
        self.viewport.queue_timer_event(node);
        self.dispatch_and_redraw(view);
    }

    fn change_event(&mut self, node: NodeId) {
        self.viewport.queue_change_event(node);
    }

    fn translation_map(&self) -> TranslationMap {
        self.viewport.get_translation_map()
    }

    fn build_accesskit_update(&mut self) -> Result<TreeUpdate, AccessKitUpdateError> {
        let state = self.app_state.borrow();
        self.viewport.build_accesskit_tree(&*state)
    }

    fn accessibility_action_event(&mut self, view: &RosinView, request: ActionRequest) {
        self.viewport.queue_accessibility_action_event(request);
        self.dispatch_and_redraw(view);
    }

    #[allow(clippy::type_complexity)]
    #[cfg(all(feature = "hot-reload", debug_assertions))]
    fn use_library(&mut self, lib: &libloading::Library) {
        let symbol = unsafe { lib.get(self.desc.viewfn.symbol) };
        let func: libloading::Symbol<fn(&S, &mut Ui<S, WindowHandle>)> = match symbol {
            Ok(s) => s,
            Err(_) => {
                error!("Hot-reload: failed to load symbol `{:?}`", self.desc.viewfn.symbol);
                return;
            }
        };
        self.viewport.set_view_callback(*func);

        if let Some(wgpufn) = self.desc.wgpufn.as_mut() {
            let symbol = unsafe { lib.get(wgpufn.symbol) };
            let func: libloading::Symbol<fn(&S, &mut WgpuCtx<'_>)> = match symbol {
                Ok(s) => s,
                Err(_) => {
                    error!("Hot-reload: failed to load symbol `{:?}`", wgpufn.symbol);
                    return;
                }
            };
            wgpufn.func = *func;
        }
    }

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    fn serializable_window_desc(&self, view: &RosinView) -> super::hot::SerializableWindowDesc {
        use super::hot::SerializableWindowDesc;
        use rosin_core::kurbo::{Point, Size};

        let (size, position) = if let Some(win) = view.window() {
            let frame = win.frame();
            let content = win.contentRectForFrameRect(frame);
            (Size::new(content.size.width, content.size.height), Some(Point::new(content.origin.x, content.origin.y)))
        } else {
            (self.desc.size, self.desc.position)
        };

        SerializableWindowDesc {
            viewfn: self.desc.viewfn.symbol.to_string(),
            wgpufn: self.desc.wgpufn.as_ref().map(|f| f.symbol.to_string()),
            title: self.desc.title.as_deref().map(|s| s.to_string()),
            menu: self.desc.menu.clone(),
            size,
            position,
            min_size: self.desc.min_size,
            max_size: self.desc.max_size,
            resizeable: self.desc.resizeable,
            close_button: self.desc.close_button,
            minimize_button: self.desc.minimize_button,
            maximize_button: self.desc.maximize_button,
        }
    }
}

impl RosinView {
    #[cfg(all(feature = "hot-reload", debug_assertions))]
    pub fn use_library(&self, new_lib: &libloading::Library) {
        self.ivars().viewport.borrow_mut().use_library(new_lib);
        self.request_redraw();
    }

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    pub fn serializable_window_desc(&self) -> super::hot::SerializableWindowDesc {
        self.ivars().viewport.borrow().serializable_window_desc(self)
    }

    pub(crate) fn set_input_handler(&self, id: Option<NodeId>, handler: Option<Box<dyn InputHandler + Send + Sync>>) {
        self.ivars().input_handler_node.set(id);
        *self.ivars().input_handler.borrow_mut() = handler;
    }

    pub(crate) fn set_main_menu(&self, desc: Option<MenuDesc>) {
        if let Some(desc) = desc {
            let translation_map = self.ivars().viewport.borrow().translation_map();
            let ns_menu = self.create_ns_menu(desc, &translation_map);
            *self.ivars().main_menu.borrow_mut() = Some(ns_menu);
        } else {
            *self.ivars().main_menu.borrow_mut() = None;
        }

        if let Some(handle) = self.ivars().handle.borrow().as_ref()
            && handle.is_active()
        {
            let mtm = unsafe { MainThreadMarker::new_unchecked() };
            let menu = self.ivars().main_menu.borrow();
            NSApp(mtm).setMainMenu(menu.as_deref());
        }
    }

    pub(crate) fn show_context_menu(&self, node: NodeId, desc: MenuDesc, pos: rosin_core::vello::kurbo::Point) {
        let translation_map = self.ivars().viewport.borrow().translation_map();
        let ns_menu = self.create_ns_menu(desc, &translation_map);

        self.ivars().context_menu_node.set(Some(node));

        if let Some(ns_view) = self.window().and_then(|w| w.contentView()) {
            let window_point = NSPoint::new(pos.x, pos.y);
            ns_menu.popUpMenuPositioningItem_atLocation_inView(None, window_point, Some(&ns_view));
        }

        self.ivars().context_menu_node.set(None);
    }

    fn set_display_link_active(&self, active: bool) {
        if let Some(dl) = self.ivars().display_link.borrow().as_deref() {
            if dl.isPaused() && active {
                // When toggling, reset last_frame so we don't get a huge delta after being paused.
                self.ivars().last_frame.set(None);
            }
            dl.setPaused(!active);
        }
    }

    fn request_redraw(&self) {
        if let Some(layer) = self.layer() {
            layer.setNeedsDisplay();
        }
    }

    fn invalidate_ime_rects(&self) {
        if let Some(input_context) = self.inputContext() {
            input_context.invalidateCharacterCoordinates();
        }
    }

    fn queue_change_event(&self) {
        if let Some(id) = self.ivars().input_handler_node.get() {
            self.ivars().viewport.borrow_mut().change_event(id);
        }
    }

    fn layout(&self) {
        let _: () = unsafe { msg_send![super(self), layout] };
        self.update_tracking_areas();
    }

    fn update_tracking_areas(&self) {
        unsafe {
            for area in self.trackingAreas().iter() {
                self.removeTrackingArea(&area);
            }

            let options = NSTrackingAreaOptions::ActiveAlways
                | NSTrackingAreaOptions::InVisibleRect
                | NSTrackingAreaOptions::MouseMoved
                | NSTrackingAreaOptions::MouseEnteredAndExited
                | NSTrackingAreaOptions::EnabledDuringMouseDrag;
            let tracking_area = NSTrackingArea::initWithRect_options_owner_userInfo(NSTrackingArea::alloc(), NSRect::ZERO, options, Some(self), None);
            self.addTrackingArea(&tracking_area);
        }
    }

    fn key_down(&self, ns_event: &NSEvent) {
        if self.ivars().input_handler.borrow().is_some() {
            self.ivars().key_down_consumed.set(false);
            let events = [ns_event];
            let events_array = NSArray::from_slice(&events);
            self.interpretKeyEvents(&events_array);

            if !self.ivars().key_down_consumed.get() {
                self.ivars().viewport.borrow_mut().keyboard_event(ns_event, self);
            }
            return;
        }

        self.ivars().viewport.borrow_mut().keyboard_event(ns_event, self);
    }

    // copy text, no need to redraw
    fn copy(&self, _sender: &AnyObject) {
        let mut handler_ref = self.ivars().input_handler.borrow_mut();
        if let Some(handler) = handler_ref.as_deref_mut() {
            handler.handle_action(Action::Copy);
        }
    }

    // cut text, queue change event, and request redraw
    fn cut(&self, _sender: &AnyObject) {
        let mut handler_ref = self.ivars().input_handler.borrow_mut();
        if let Some(handler) = handler_ref.as_deref_mut()
            && handler.handle_action(Action::Cut)
        {
            self.queue_change_event();
            self.ivars().viewport.borrow_mut().dispatch_and_redraw(self);
        }
    }

    // paste text, queue change event, and request redraw
    fn paste(&self, _sender: &AnyObject) {
        let mut handler_ref = self.ivars().input_handler.borrow_mut();
        if let Some(handler) = handler_ref.as_deref_mut()
            && handler.handle_action(Action::Paste)
        {
            self.queue_change_event();
            self.ivars().viewport.borrow_mut().dispatch_and_redraw(self);
        }
    }

    // select text and request redraw
    fn select_all(&self, _sender: &AnyObject) {
        let mut handler_ref = self.ivars().input_handler.borrow_mut();
        if let Some(handler) = handler_ref.as_deref_mut()
            && handler.handle_action(Action::Select(SelectionUnit::All))
        {
            self.ivars().viewport.borrow_mut().dispatch_and_redraw(self);
        }
    }

    fn insert_text(&self, string: &AnyObject, replacement_range: NSRange) {
        self.ivars().key_down_consumed.set(true);

        let mut handler_ref = self.ivars().input_handler.borrow_mut();
        if let Some(handler) = handler_ref.as_deref_mut() {
            let text = util::extract_string(string);
            let text_len = text.len();

            // Determine the range in the document to overwrite.
            let range = if replacement_range.location != NSNotFound as usize {
                handler
                    .utf16_range_to_utf8_range(util::range_from_ns(replacement_range))
                    .unwrap_or_else(|| handler.selection())
            } else {
                handler.composition_range().unwrap_or_else(|| handler.selection())
            };

            let start = range.start;
            handler.replace_range(range, &text);

            // Update selection to end of inserted text to prevent backward typing
            let new_cursor_pos = start + text_len;
            handler.set_selection(new_cursor_pos..new_cursor_pos);

            // Text is committed, so there is no longer "marked" text.
            handler.set_composition_range(None);
        }

        self.queue_change_event();
        self.invalidate_ime_rects();
        self.ivars().viewport.borrow_mut().dispatch_and_redraw(self);
    }

    fn do_command_by_selector(&self, selector: Sel) {
        let mut handler_ref = self.ivars().input_handler.borrow_mut();
        let Some(handler) = handler_ref.as_deref_mut() else {
            return;
        };

        let Some(action) = util::selector_to_action(selector) else {
            return;
        };

        if !handler.handle_action(action) {
            return;
        }

        self.ivars().key_down_consumed.set(true);

        if action.edits_text() {
            self.queue_change_event();
        }

        self.invalidate_ime_rects();
        self.ivars().viewport.borrow_mut().dispatch_and_redraw(self);
    }

    fn set_marked_text(&self, string: &AnyObject, selected_range: NSRange, replacement_range: NSRange) {
        // Ensure we always update IME/candidate window geometry after any marked-text change.
        self.ivars().key_down_consumed.set(true);

        let mut handler_ref = self.ivars().input_handler.borrow_mut();
        let Some(handler) = handler_ref.as_deref_mut() else {
            return;
        };

        let text = util::extract_string(string);

        // If Cocoa provided an explicit replacementRange, respect it.
        let explicit_replacement = (replacement_range.location != NSNotFound as usize)
            .then(|| handler.utf16_range_to_utf8_range(util::range_from_ns(replacement_range)))
            .flatten();

        // Determine the document range to replace.
        let range = explicit_replacement
            .or_else(|| handler.composition_range())
            .unwrap_or_else(|| handler.selection());

        let start = range.start;

        // Treat empty marked text as "end/cancel composition".
        if text.is_empty() {
            // If we were actually composing, delete the current composition contents.
            if handler.composition_range().is_some() {
                handler.replace_range(range, "");
            }

            // End composition and place caret at the start of the affected range.
            handler.set_composition_range(None);
            handler.set_selection(start..start);
        } else {
            // Normal pre-edit update.
            handler.replace_range(range, &text);

            // New composition range becomes exactly the inserted preedit string.
            let new_comp_start = start;
            let new_comp_end = new_comp_start + text.len();
            let new_comp_range = new_comp_start..new_comp_end;
            handler.set_composition_range(Some(new_comp_range));

            // selected_range is relative to the marked string; convert to absolute.
            let sel_absolute = (selected_range.location != NSNotFound as usize)
                .then(|| util::local_utf16_to_utf8(&text, util::range_from_ns(selected_range)))
                .flatten()
                .map(|rel| (new_comp_start + rel.start)..(new_comp_start + rel.end));

            if let Some(sel_absolute) = sel_absolute {
                handler.set_selection(sel_absolute);
            } else {
                handler.set_selection(new_comp_end..new_comp_end);
            }
        }

        self.invalidate_ime_rects();
        self.ivars().viewport.borrow_mut().dispatch_and_redraw(self);
    }

    fn unmark_text(&self) {
        self.ivars().key_down_consumed.set(true);

        let mut handler_ref = self.ivars().input_handler.borrow_mut();
        if let Some(handler) = handler_ref.as_deref_mut() {
            handler.set_composition_range(None);
        }
        self.invalidate_ime_rects();
        self.ivars().viewport.borrow_mut().dispatch_and_redraw(self);
    }

    fn selected_range(&self) -> NSRange {
        let handler_ref = self.ivars().input_handler.borrow();
        if let Some(handler) = handler_ref.as_deref() {
            let range = handler.selection();
            let len_utf16_start = handler.utf8_range_utf16_len(0..range.start).unwrap_or(0);
            let len_utf16_sel = handler.utf8_range_utf16_len(range).unwrap_or(0);
            NSRange::new(len_utf16_start, len_utf16_sel)
        } else {
            NSRange::new(NSNotFound as usize, 0)
        }
    }

    fn marked_range(&self) -> NSRange {
        let handler_ref = self.ivars().input_handler.borrow();
        if let Some(handler) = handler_ref.as_deref()
            && let Some(range) = handler.composition_range()
        {
            let len_utf16_start = handler.utf8_range_utf16_len(0..range.start).unwrap_or(0);
            let len_utf16_sel = handler.utf8_range_utf16_len(range).unwrap_or(0);
            return NSRange::new(len_utf16_start, len_utf16_sel);
        }
        NSRange::new(NSNotFound as usize, 0)
    }

    fn has_marked_text(&self) -> Bool {
        let handler_ref = self.ivars().input_handler.borrow();
        if let Some(handler) = handler_ref.as_deref() {
            Bool::new(handler.composition_range().is_some())
        } else {
            Bool::new(false)
        }
    }

    fn attributed_substring(&self, proposed_range: NSRange, actual_range: *mut NSRange) -> *mut NSAttributedString {
        let handler_ref = self.ivars().input_handler.borrow();
        if let Some(handler) = handler_ref.as_deref()
            && let Some(range) = handler.utf16_range_to_utf8_range(util::range_from_ns(proposed_range))
        {
            let text = handler.slice(range.clone());
            let ns_string = NSString::from_str(&text);
            let attr_string = NSAttributedString::initWithString(NSAttributedString::alloc(), &ns_string);

            if !actual_range.is_null() {
                let len_utf16_start = handler.utf8_range_utf16_len(0..range.start).unwrap_or(0);
                let len_utf16_range = handler.utf8_range_utf16_len(range).unwrap_or(0);
                unsafe { *actual_range = NSRange::new(len_utf16_start, len_utf16_range) };
            }

            return Retained::autorelease_ptr(attr_string);
        }
        std::ptr::null_mut()
    }

    fn valid_attributes_for_marked_text(&self) -> *mut NSArray<NSString> {
        let array = NSArray::<NSString>::new();
        Retained::autorelease_ptr(array)
    }

    fn first_rect_for_character_range(&self, range: NSRange, actual_range: *mut NSRange) -> NSRect {
        let handler_ref = self.ivars().input_handler.borrow();
        if let Some(handler) = handler_ref.as_deref()
            && let Some(utf8_range) = handler.utf16_range_to_utf8_range(util::range_from_ns(range))
            && let Some(rect) = handler.bounding_box_for_range(utf8_range.clone())
        {
            if !actual_range.is_null() {
                // Update actualRange to reflect the UTF-8 range used, converted back to UTF-16
                // This ensures that if utf16_range_to_utf8_range snapped to a valid boundary,
                // we report the correct range to the IME.
                let len_utf16_start = handler.utf8_range_utf16_len(0..utf8_range.start).unwrap_or(0);
                let len_utf16_len = handler.utf8_range_utf16_len(utf8_range).unwrap_or(0);
                unsafe { *actual_range = NSRange::new(len_utf16_start, len_utf16_len) };
            }

            let view_rect = NSRect::new(NSPoint::new(rect.x0, rect.y0), NSSize::new(rect.width(), rect.height()));

            // Note: convertRect:toView:nil converts to Window coordinates.
            // Since isFlipped is true, this should handle the Y-flip logic correctly
            // assuming the window/screen conversion also respects it.
            let win_rect = self.convertRect_toView(view_rect, None);
            if let Some(win) = self.window() {
                return win.convertRectToScreen(win_rect);
            }
        }

        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0))
    }

    fn character_index_for_point(&self, point: NSPoint) -> usize {
        let handler_ref = self.ivars().input_handler.borrow();
        if let Some(handler) = handler_ref.as_deref()
            && let Some(win) = self.window()
        {
            let win_rect = NSRect::new(point, NSSize::new(0.0, 0.0));
            let win_point = win.convertRectFromScreen(win_rect).origin;
            let view_point = self.convertPoint_fromView(win_point, None);
            let cursor_point = rosin_core::vello::kurbo::Point::new(view_point.x, view_point.y);

            if let Some(cursor) = handler.hit_test_point(cursor_point) {
                return handler.utf8_range_utf16_len(0..cursor.index()).unwrap_or(NSNotFound as usize);
            }
        }
        NSNotFound as usize
    }

    fn create_ns_menu(&self, desc: MenuDesc, translation_map: &TranslationMap) -> Retained<NSMenu> {
        let mtm = MainThreadMarker::from(self);
        let ns_menu = NSMenu::new(mtm);
        ns_menu.setAutoenablesItems(false);

        for item in &*desc.items {
            match item {
                MenuItem::Action {
                    title,
                    command,
                    shortcut,
                    enabled,
                    ..
                } => {
                    let title_str = title.resolve(translation_map).to_string();
                    let key_equiv = if let Some(hotkey) = &shortcut {
                        match &hotkey.key {
                            Key::Character(s) => s.to_lowercase(),
                            _ => String::new(),
                        }
                    } else {
                        String::new()
                    };

                    unsafe {
                        let ns_item = NSMenuItem::initWithTitle_action_keyEquivalent(
                            NSMenuItem::alloc(mtm),
                            &NSString::from_str(&title_str),
                            Some(sel!(menuItemClicked:)),
                            &NSString::from_str(&key_equiv),
                        );

                        if let Some(hotkey) = shortcut {
                            let mut mask = NSEventModifierFlags::empty();
                            if hotkey.mods.contains(Modifiers::META) {
                                mask |= NSEventModifierFlags::Command;
                            }
                            if hotkey.mods.contains(Modifiers::SHIFT) {
                                mask |= NSEventModifierFlags::Shift;
                            }
                            if hotkey.mods.contains(Modifiers::ALT) {
                                mask |= NSEventModifierFlags::Option;
                            }
                            if hotkey.mods.contains(Modifiers::CONTROL) {
                                mask |= NSEventModifierFlags::Control;
                            }
                            ns_item.setKeyEquivalentModifierMask(mask);
                        } else {
                            ns_item.setKeyEquivalentModifierMask(NSEventModifierFlags::empty());
                        }

                        ns_item.setTag(command.0 as isize);
                        ns_item.setEnabled(*enabled);
                        ns_menu.addItem(&ns_item);
                    }
                }
                MenuItem::Submenu { title, menu, enabled } => {
                    let sub = self.create_ns_menu(menu.clone(), translation_map);
                    let title_str = title.resolve(translation_map).to_string();
                    let ns_title = NSString::from_str(&title_str);
                    let item = NSMenuItem::new(mtm);
                    item.setTitle(&ns_title);
                    item.setSubmenu(Some(&sub));
                    item.setEnabled(*enabled);
                    ns_menu.addItem(&item);
                }
                MenuItem::Standard(action) => {
                    let (title, sel, key, mods) = match action {
                        StandardAction::Copy => ("Copy", sel!(copy:), "c", NSEventModifierFlags::Command),
                        StandardAction::Cut => ("Cut", sel!(cut:), "x", NSEventModifierFlags::Command),
                        StandardAction::Paste => ("Paste", sel!(paste:), "v", NSEventModifierFlags::Command),
                        StandardAction::SelectAll => ("Select All", sel!(selectAll:), "a", NSEventModifierFlags::Command),
                    };

                    unsafe {
                        let item = NSMenuItem::initWithTitle_action_keyEquivalent(
                            NSMenuItem::alloc(mtm),
                            &NSString::from_str(title),
                            Some(sel),
                            &NSString::from_str(key),
                        );
                        item.setKeyEquivalentModifierMask(mods);
                        ns_menu.addItem(&item);
                    }
                }
                MenuItem::Separator => ns_menu.addItem(&NSMenuItem::separatorItem(mtm)),
            }
        }
        ns_menu
    }
}

define_class!(
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ViewIvars]
    pub(crate) struct RosinView;

    unsafe impl NSObjectProtocol for RosinView {}

    unsafe impl CALayerDelegate for RosinView {
        /// Tells the delegate to display the contents of the layer.
        ///
        /// Parameter:
        /// * `layer` – The layer that needs to be displayed.
        ///
        /// If implemented, this method is called by the layer's default `display` method implementation.
        /// In your implementation, you should update the layer's contents (typically by setting the layer's `contents` property).
        #[unsafe(method(displayLayer:))]
        fn __display_layer(&self, _layer: &CALayer) {
            // Looks like this can be called multiple times per frame.
            self.ivars().viewport.borrow_mut().update_layer(self);
        }
    }

    unsafe impl NSWindowDelegate for RosinView {
        /// Tells the delegate that the user has attempted to close a window (or the window received a `performClose:` message).
        ///
        /// Parameter:
        /// * `sender` – The `NSWindow` that is attempting to close.
        ///
        /// Return Value: `YES` to allow `sender` to close; otherwise `NO`.
        ///
        /// You can implement this method to decide whether the window should close (for example, by prompting the user
        /// to savechanges). If you return `NO`, the window remains open.
        #[unsafe(method(windowShouldClose:))]
        fn __window_should_close(&self, _sender: &NSWindow) -> bool {
            self.ivars().viewport.borrow_mut().close(self)
        }

        /// Tells the delegate that the window's backing properties changed.
        ///
        /// Parameter:
        /// * `notification` – An `NSNotification` indicating the change in the window's backing properties.
        ///
        /// This method is invoked when the window's backing scale factor or color space changes
        /// (for example, when moving the window to a display with a different resolution or color profile).
        #[unsafe(method(windowDidChangeBackingProperties:))]
        fn __window_did_change_backing_properties(&self, _notification: &NSNotification) {
            self.ivars().viewport.borrow_mut().window_event(WindowEvent::DidChangeBackingProperties, self);
        }

        /// Tells the delegate that the window has become the key window.
        ///
        /// Parameter:
        /// * `notification` – An `NSNotification` sent when the window became key.
        /// The notification's object is the window that became key.
        #[unsafe(method(windowDidBecomeKey:))]
        fn __window_did_become_key(&self, _notification: &NSNotification) {
            self.ivars().viewport.borrow_mut().window_event(WindowEvent::DidBecomeKey, self);
        }

        /// Tells the delegate that the window has resigned key window status.
        ///
        /// Parameter:
        /// * `notification` – An `NSNotification` sent when the window resigned key status.
        #[unsafe(method(windowDidResignKey:))]
        fn __window_did_resign_key(&self, _notification: &NSNotification) {
            self.ivars().viewport.borrow_mut().window_event(WindowEvent::DidResignKey, self);
        }
    }

    unsafe impl NSTextInputClient for RosinView {
        /// Inserts the given string into the receiver, replacing the specified content.
        ///
        /// Parameters:
        /// * `string` – The text to insert, either an `NSString` or `NSAttributedString` instance.
        /// * `replacement_range` – The range of content to replace in the receiver's text storage.
        ///
        /// This method is the entry point for inserting text typed by the user and is generally not suitable
        /// for other purposes. Programmatic modification of the text is best done by operating on the text storage
        /// directly. Because this method pertains to the actions of the user, the text view must be editable
        /// for the insertion to work.
        #[unsafe(method(insertText:replacementRange:))]
        fn __insert_text(&self, string: &AnyObject, replacement_range: NSRange) {
            self.insert_text(string, replacement_range);
        }

        /// Invokes the action specified by the given selector.
        ///
        /// Parameter:
        /// * `selector` – The selector to invoke.
        ///
        /// If `selector` cannot be invoked, then `doCommandBySelector:` should not pass this message up the
        /// responder chain. `NSResponder` also implements this method and forwards uninvokable commands up the
        /// responder chain, but a text view should not.  A text view implementing `NSTextInputClient` inherits
        /// from `NSView`, which inherits from `NSResponder`, so your implementation overrides the one in
        /// `NSResponder` and should not call `super`.
        #[unsafe(method(doCommandBySelector:))]
        fn __do_command_by_selector(&self, selector: Sel) {
            self.do_command_by_selector(selector);
        }

        /// Replaces a specified range in the receiver's text storage with the given string and sets the selection.
        ///
        /// Parameters:
        /// * `string` – The string to insert. Can be either an `NSString` or `NSAttributedString` instance.
        /// * `selected_range` – The range to set as the selection, computed from the beginning of the inserted string.
        /// * `replacement_range` – The range to replace, computed from the beginning of the marked text.
        ///
        /// If there is no marked text, the current selection is replaced. If there is no selection, the string is
        /// inserted at the insertion point. When `string` is an `NSString` object, the receiver is expected to render
        /// the marked text with distinguishing appearance (for example, `NSTextView` uses `markedTextAttributes`)
        #[unsafe(method(setMarkedText:selectedRange:replacementRange:))]
        fn __set_marked_text(&self, string: &AnyObject, selected_range: NSRange, replacement_range: NSRange) {
            self.set_marked_text(string, selected_range, replacement_range);
        }

        /// Unmarks the marked text.
        ///
        /// The receiver removes any marking from pending input text and disposes of the marked text as it wishes.
        /// The text view should accept the marked text as if it had been inserted normally.  If there is no marked
        /// text, the invocation of this method has no effect.
        #[unsafe(method(unmarkText))]
        fn __unmark_text(&self) {
            self.unmark_text();
        }

        /// Returns the range of selected text.
        ///
        /// Return Value: The range of selected text or `{NSNotFound, 0}` if there is no selection.  The returned range
        /// measures from the start of the receiver's text storage—that is, from 0 to the document length.
        #[unsafe(method(selectedRange))]
        fn __selected_range(&self) -> NSRange {
            self.selected_range()
        }

        /// Returns the range of the marked text.
        ///
        /// Return Value: The range of marked text or `{NSNotFound, 0}` if there is no marked range.  The returned range
        /// measures from the start of the receiver's text storage.  The return value's `location` is `NSNotFound`
        /// and its `length` is `0` if and only if `hasMarkedText` returns `NO`.
        #[unsafe(method(markedRange))]
        fn __marked_range(&self) -> NSRange {
            self.marked_range()
        }

        /// Returns a Boolean value indicating whether the receiver has marked text.
        ///
        /// Return Value: `YES` if the receiver has marked text; otherwise `NO`.  The text view itself may call this method
        /// to determine whether there currently is marked text.  `NSTextView`, for example, disables the Edit > Copy
        /// menu item when this method returns `YES`.
        #[unsafe(method(hasMarkedText))]
        fn __has_marked_text(&self) -> Bool {
            self.has_marked_text()
        }

        /// Returns an attributed string derived from the given range in the receiver's text storage.
        ///
        /// Parameters:
        /// * `proposed_range` – The range in the text storage from which to create the returned string.
        /// * `actual_range` – The actual range of the returned string if it was adjusted—for example, to a grapheme
        ///   cluster boundary or for performance or other reasons. `NULL` if the range was not adjusted.
        ///
        /// Return Value: The string created from the given range.  May return `nil`.
        ///
        /// An implementation of this method should be prepared for `proposed_range` to be out of bounds.  If the
        /// requested range extends beyond the document's range, return the intersection of the two ranges; if it lies
        /// completely outside the document's range, return `nil`.
        #[unsafe(method(attributedSubstringForProposedRange:actualRange:))]
        fn __attributed_substring(&self, proposed_range: NSRange, actual_range: *mut NSRange) -> *mut NSAttributedString {
            self.attributed_substring(proposed_range, actual_range)
        }

        /// Returns an array of attribute names recognized by the receiver.
        ///
        /// Return Value: An array of `NSString` objects representing names for the supported attributes.  Returns an
        /// empty array if no attributes are supported.  See the `NSAttributedString` Application Kit Additions
        /// Reference for the set of string constants representing standard attributes.
        #[unsafe(method(validAttributesForMarkedText))]
        fn __valid_attributes_for_marked_text(&self) -> *mut NSArray<NSString> {
            self.valid_attributes_for_marked_text()
        }

        /// Returns the first logical boundary rectangle for characters in the given range.
        ///
        /// Parameters:
        /// * `range` – The character range whose boundary rectangle is returned.
        /// * `actual_range` – If non-`NULL`, contains the character range corresponding to the returned area if it was
        ///   adjusted, for example, to a grapheme cluster boundary or to characters in the first line fragment.
        ///
        /// Return Value: The boundary rectangle for the given range of characters, in screen coordinates.  The rectangle's
        /// `size` value can be negative if the text flows to the left.
        ///
        /// If `range` spans multiple lines of text in the text view, the rectangle returned is the one surrounding the
        /// characters in the first line; in that case, `actual_range` contains the range covered by that first rect, so
        /// you can query subsequent line fragments by calling this method repeatedly.  If the length of `range` is 0
        /// (for example, if nothing is selected at the insertion point), the rectangle coincides with the insertion
        /// point and its width is 0.
        #[unsafe(method(firstRectForCharacterRange:actualRange:))]
        fn __first_rect_for_character_range(&self, range: NSRange, actual_range: *mut NSRange) -> NSRect {
            self.first_rect_for_character_range(range, actual_range)
        }

        /// Returns the index of the character whose bounding rectangle includes the given point.
        ///
        /// Parameter:
        /// * `point` – The point to test, in screen coordinates.
        ///
        /// Return Value: The character index, measured from the start of the receiver's text storage, of the character
        /// containing the given point.  Returns `NSNotFound` if the cursor is not within a character's bounding
        /// rectangle.
        #[unsafe(method(characterIndexForPoint:))]
        fn __character_index_for_point(&self, point: NSPoint) -> usize {
            self.character_index_for_point(point)
        }
    }

    impl RosinView {
        // Note: custom method not in NSView
        #[unsafe(method(anim_frame:))]
        fn __anim_frame(&self, _sender: &CADisplayLink) {
            self.ivars().viewport.borrow_mut().anim_frame(self);
        }

        // Note: custom method not in NSView
        #[unsafe(method(reload_assets))]
        fn __reload_assets(&self) {
            self.ivars().viewport.borrow_mut().reload_assets(self);
        }

        // Note: custom method not in NSView
        #[unsafe(method(menuItemClicked:))]
        fn __menu_item_clicked(&self, sender: &NSMenuItem) {
            let tag = sender.tag();
            let node = self.ivars().context_menu_node.get();
            self.ivars().viewport.borrow_mut().command_event(self, node, CommandId(tag as u32));
        }

        /// Returns a Boolean value indicating whether the view uses a flipped coordinate system.
        ///
        /// Return Value: `YES` if the view's coordinate system is flipped (with the origin at the top-left); otherwise `NO`.
        ///
        /// By default, NSView returns `NO` (origin at the bottom-left).
        /// Override this method to return `YES` for a viewthat draws content with a top-left origin.
        #[unsafe(method(isFlipped))]
        fn __is_flipped(&self) -> bool {
            true
        }

        /// Returns a Boolean value indicating whether the receiver accepts first responder status.
        ///
        /// Return Value: `YES` if the receiver can become the first responder; otherwise `NO`.
        ///
        /// The default NSView implementation returns `NO`. Subclasses should override this method and
        /// return `YES` ifthe view should be able to become first responder (for example, to handle keyboard input).
        #[unsafe(method(acceptsFirstResponder))]
        fn __accepts_first_responder(&self) -> bool {
            true
        }

        /// Draws the view's contents within the specified rectangle.
        ///
        /// Parameter:
        /// * `rect` – The portion of the view's bounds that needs to be redrawn.
        ///
        /// You should not call this method directly. It is invoked by the system when the view is
        /// marked as needing display or during an update cycle. Subclasses should override this method
        /// to perform any custom drawing of the view's content.
        #[unsafe(method(drawRect:))]
        fn __draw_rect(&self, _: NSRect) {
            // Looks like `displayLayer:` is only called if there's a `drawRect:` method, even though it's never called.
        }

        /// Returns a Boolean value indicating whether the view accepts the first mouse event.
        ///
        /// Parameter:
        /// * `event` – The mouse-down event that triggered the attempt to activate the window.
        ///             (This parameter is ignored by the default implementation.)
        ///
        /// Return Value: `YES` if the view should receive the mouse-down event even when its window is inactive; otherwise `NO`.
        #[unsafe(method(acceptsFirstMouse:))]
        fn __accepts_first_mouse(&self, _: &NSEvent) -> bool {
            true
        }

        /// Updates the view's tracking areas.
        ///
        /// This method is invoked automatically when the view's geometry changes such that its tracking areas need
        /// to be recalculated. An override of this method should remove any outdated tracking areas and add any
        /// needed new tracking areas, then call `super.updateTrackingAreas()`.
        #[unsafe(method(updateTrackingAreas))]
        fn __update_tracking_areas(&self) {
            self.update_tracking_areas();
        }

        /// Adjusts the layout of the view's subviews.
        ///
        /// This method is called automatically during the layout pass when the view's bounds or constraints change.
        /// Subclasses can override this method to reposition or resize subviews as needed.
        #[unsafe(method(layout))]
        fn __layout(&self) {
            self.layout();
        }

        /// Informs the receiver that the user has pressed the left mouse button in the view.
        ///
        /// Parameter:
        /// * `ns_event` – The mouse-down event.
        ///
        /// The default implementation simply passes this message to the next responder.
        #[unsafe(method(mouseDown:))]
        fn __mouse_down(&self, ns_event: &NSEvent) {
            self.ivars().viewport.borrow_mut().input_event(PointerEvent::Down, ns_event, self);
        }

        /// Informs the receiver that the user has released the left mouse button.
        ///
        /// Parameter:
        /// * `ns_event` – The mouse-up event.
        ///
        /// The default implementation simply passes this message to the next responder.
        #[unsafe(method(mouseUp:))]
        fn __mouse_up(&self, ns_event: &NSEvent) {
            self.ivars().viewport.borrow_mut().input_event(PointerEvent::Up, ns_event, self);
        }

        /// Informs the receiver that the user dragged the mouse with the left button held down.
        ///
        /// Parameter:
        /// * `ns_event` – The mouse-dragged event (left button).
        ///
        /// The default implementation simply passes this message to the next responder.
        #[unsafe(method(mouseDragged:))]
        fn __mouse_dragged(&self, ns_event: &NSEvent) {
            self.ivars().viewport.borrow_mut().input_event(PointerEvent::Move, ns_event, self);
        }

        /// Informs the receiver that the user has pressed the right mouse button.
        ///
        /// Parameter:
        /// * `ns_event` – The right-mouse-down event.
        ///
        /// The default implementation simply passes this message to the next responder.
        #[unsafe(method(rightMouseDown:))]
        fn __right_mouse_down(&self, ns_event: &NSEvent) {
            self.ivars().viewport.borrow_mut().input_event(PointerEvent::Down, ns_event, self);
        }

        /// Informs the receiver that the user has released the right mouse button.
        ///
        /// Parameter:
        /// * `ns_event` – The right-mouse-up event.
        ///
        /// The default implementation simply passes this message to the next responder.
        #[unsafe(method(rightMouseUp:))]
        fn __right_mouse_up(&self, ns_event: &NSEvent) {
            self.ivars().viewport.borrow_mut().input_event(PointerEvent::Up, ns_event, self);
        }

        /// Informs the receiver that the user dragged the mouse with the right button held down.
        ///
        /// Parameter:
        /// * `ns_event` – The right-mouse-dragged event.
        ///
        /// The default implementation simply passes this message to the next responder.
        #[unsafe(method(rightMouseDragged:))]
        fn __right_mouse_dragged(&self, ns_event: &NSEvent) {
            self.ivars().viewport.borrow_mut().input_event(PointerEvent::Move, ns_event, self);
        }

        /// Informs the receiver that the user has pressed an auxiliary mouse button (neither left nor right).
        ///
        /// Parameter:
        /// * `ns_event` – The other-mouse-down event.
        ///
        /// The default implementation simply passes this message to the next responder.
        #[unsafe(method(otherMouseDown:))]
        fn __other_mouse_down(&self, ns_event: &NSEvent) {
            self.ivars().viewport.borrow_mut().input_event(PointerEvent::Down, ns_event, self);
        }

        /// Informs the receiver that the user has released an auxiliary mouse button.
        ///
        /// Parameter:
        /// * `ns_event` – The other-mouse-up event.
        ///
        /// The default implementation simply passes this message to the next responder.
        #[unsafe(method(otherMouseUp:))]
        fn __other_mouse_up(&self, ns_event: &NSEvent) {
            self.ivars().viewport.borrow_mut().input_event(PointerEvent::Up, ns_event, self);
        }

        /// Informs the receiver that the user dragged the mouse with an auxiliary button held down.
        ///
        /// Parameter:
        /// * `ns_event` – The other-mouse-dragged event.
        ///
        /// The default implementation simply passes this message to the next responder.
        #[unsafe(method(otherMouseDragged:))]
        fn __other_mouse_dragged(&self, ns_event: &NSEvent) {
            self.ivars().viewport.borrow_mut().input_event(PointerEvent::Move, ns_event, self);
        }

        /// Informs the receiver that the mouse cursor has moved within the view (with no buttons pressed).
        ///
        /// Parameter:
        /// * `ns_event` – The mouse-moved event.
        ///
        /// The default implementation simply passes this message to the next responder.
        #[unsafe(method(mouseMoved:))]
        fn __mouse_moved(&self, ns_event: &NSEvent) {
            self.ivars().viewport.borrow_mut().input_event(PointerEvent::Move, ns_event, self);
        }

        /// Informs the receiver that the mouse cursor has exited the view's boundary.
        ///
        /// Parameter:
        /// * `ns_event` – The mouse-exited event.
        ///
        /// The default implementation simply passes this message to the next responder.
        #[unsafe(method(mouseExited:))]
        fn __mouse_exited(&self, ns_event: &NSEvent) {
            self.ivars().viewport.borrow_mut().input_event(PointerEvent::Leave, ns_event, self);
        }

        /// Informs the receiver that the mouse cursor has entered the view's boundary.
        ///
        /// Parameter:
        /// * `ns_event` – The mouse-entered event.
        ///
        /// The default implementation simply passes this message to the next responder.
        #[unsafe(method(mouseEntered:))]
        fn __mouse_entered(&self, ns_event: &NSEvent) {
            self.ivars().viewport.borrow_mut().input_event(PointerEvent::Move, ns_event, self);
        }

        /// Informs the receiver of a scroll-wheel event.
        ///
        /// Parameter:
        /// * `ns_event` – The scroll-wheel event (from a mouse or trackpad).
        ///
        /// The default implementation simply passes this message to the next responder.
        #[unsafe(method(scrollWheel:))]
        fn __scroll_wheel(&self, ns_event: &NSEvent) {
            self.ivars().viewport.borrow_mut().input_event(PointerEvent::Wheel, ns_event, self);
        }

        /// Informs the receiver that the user pressed a key (key-down event).
        ///
        /// Parameter:
        /// * `ns_event` – The key-down event.
        ///
        /// The default implementation of this method passes the event to the next responder.
        /// If no responder handles the key event, the system may beep to indicate an unhandled key press.
        #[unsafe(method(keyDown:))]
        fn __key_down(&self, ns_event: &NSEvent) {
            self.key_down(ns_event);
        }

        /// Informs the receiver that the user released a key (key-up event).
        ///
        /// Parameter:
        /// * `ns_event` – The key-up event.
        ///
        /// The default implementation simply passes this message to the next responder.
        #[unsafe(method(keyUp:))]
        fn __key_up(&self, ns_event: &NSEvent) {
            self.ivars().viewport.borrow_mut().keyboard_event(ns_event, self);
        }

        /// Informs the receiver that the state of the modifier keys changed.
        ///
        /// Parameter:
        /// * `ns_event` – An event representing the change in modifier keys (e.g., Shift, Control, Option, Command).
        ///
        /// The default implementation simply passes this message to the next responder.
        #[unsafe(method(flagsChanged:))]
        fn __flags_changed(&self, ns_event: &NSEvent) {
            self.ivars().viewport.borrow_mut().keyboard_event(ns_event, self);
        }

        /// Copies the selected content onto the general pasteboard, in as many formats as the receiver supports.
        ///
        /// Parameter:
        /// * `sender` – The object that initiated the copy action.
        #[unsafe(method(copy:))]
        fn __copy(&self, sender: &AnyObject) {
            self.copy(sender);
        }

        /// Removes the selected content and writes it to the general pasteboard.
        ///
        /// Parameter:
        /// * `sender` – The object that initiated the cut action.
        ///
        /// This action copies the current selection to the pasteboard and then deletes the selection from the view.
        #[unsafe(method(cut:))]
        fn __cut(&self, sender: &AnyObject) {
            self.cut(sender);
        }

        /// Inserts the contents of the pasteboard at the insertion point, replacing the current selection if there is one.
        ///
        /// Parameter:
        /// * `sender` – The object that initiated the paste action.
        #[unsafe(method(paste:))]
        fn __paste(&self, sender: &AnyObject) {
            self.paste(sender);
        }

        /// Selects all content in the receiver.
        ///
        /// Parameter:
        /// * `sender` – The object that initiated the select-all action.
        #[unsafe(method(selectAll:))]
        fn __select_all(&self, sender: &AnyObject) {
            self.select_all(sender);
        }
    }
);
