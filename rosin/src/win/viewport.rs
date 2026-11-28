use std::{cell::RefCell, rc::Rc};

use crate::{kurbo::Vec2, prelude::*};
use rosin_core::{vello, viewport::Viewport, wgpu};
use windows::Win32::Foundation::HWND;

pub(crate) struct ViewportContainer<S: Sync + 'static> {
    viewport: Viewport<S, WindowHandle>,
    app_state: Rc<RefCell<S>>,
    gpu_ctx: Rc<GpuCtx>,
    vello_renderer: Rc<RefCell<vello::Renderer>>,
    surface: Option<wgpu::Surface<'static>>,
    vello_texture: Option<wgpu::Texture>,
    surface_format: Option<wgpu::TextureFormat>,
    needs_config: bool,
}

pub(crate) trait ViewportTrait {
    fn init_viewport(&mut self, handle: &WindowHandle);
    fn render(&mut self, handle: &WindowHandle);
}

impl<S: Sync + 'static> ViewportContainer<S> {
    pub(crate) fn new(
        desc: &WindowDesc<S>,
        app_state: Rc<RefCell<S>>,
        translations: TranslationMap,
        gpu_ctx: Rc<GpuCtx>,
        vello_renderer: Rc<RefCell<vello::Renderer>>,
    ) -> Self {
        Self {
            viewport: Viewport::new(desc.viewfn, desc.size, Vec2::new(1.0, 1.0), translations),
            app_state,
            needs_config: true,
            gpu_ctx,
            vello_renderer,
            surface: None,
            surface_format: None,
            vello_texture: None,
        }
    }
}

impl<S: Sync + 'static> ViewportTrait for ViewportContainer<S> {
    fn init_viewport(&mut self, handle: &WindowHandle) {
        let mut state = self.app_state.borrow_mut();
        let _ = self.viewport.frame(&*state);
        let _ = self.viewport.dispatch_event_queue(&mut state, handle);
    }

    fn render(&mut self, handle: &WindowHandle) {
        let hwnd = handle.0.view.try_on_thread(|v| v.hwnd()).unwrap();

        let size = get_client_size(hwnd);
        if size == (0, 0) {
            return;
        };

        if self.surface.is_none() {
            self.surface = match self.gpu_ctx.instance.create_surface(handle.clone()) {
                Ok(s) => Some(s),
                Err(e) => todo!("could not create surface; err info: {e:?}"),
            }
        };

        let Some(surface) = self.surface.as_ref() else { return };

        if self.needs_config {
            let capabilities = surface.get_capabilities(&self.gpu_ctx.adapter);

            let Some(format) = capabilities
                .formats
                .into_iter()
                .find(|f| matches!(f, wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm))
            else {
                return;
            };

            self.surface_format = Some(format);

            let alpha_mode = if capabilities
                .alpha_modes
                .into_iter()
                .find(|mode| mode == &wgpu::CompositeAlphaMode::PostMultiplied)
                .is_some()
            {
                wgpu::CompositeAlphaMode::PostMultiplied
            } else {
                wgpu::CompositeAlphaMode::Opaque
            };

            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                color_space: wgpu::SurfaceColorSpace::Auto,
                width: size.0,
                height: size.1,
                present_mode: wgpu::PresentMode::AutoVsync,
                alpha_mode,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };

            surface.configure(&self.gpu_ctx.device, &config);

            self.vello_texture = Some(self.gpu_ctx.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }));

            self.needs_config = false;
        }

        let texture = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex) | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
            _ => {
                self.needs_config = true;
                return;
            }
        };

        let mut state = self.app_state.borrow_mut();
        let scene = self.viewport.frame(&state);

        let Some(vello_texture) = self.vello_texture.as_ref() else { return };
        let vello_view = vello_texture.create_view(&Default::default());

        let params = vello::RenderParams {
            base_color: vello::peniko::Color::TRANSPARENT,
            width: size.0,
            height: size.1,
            antialiasing_method: vello::AaConfig::Msaa16,
        };

        let res = self
            .vello_renderer
            .borrow_mut()
            .render_to_texture(&self.gpu_ctx.device, &self.gpu_ctx.queue, scene, &vello_view, &params);
        if res.is_err() {
            eprintln!("failed to render");
            self.needs_config = true;
            return;
        }

        let surface_view = texture.texture.create_view(&Default::default());

        let mut encoder = self.gpu_ctx.device.create_command_encoder(&wgpu::wgt::CommandEncoderDescriptor::default());

        let mut blitter = self.gpu_ctx.compositor.blitter.borrow_mut();
        let blitter = blitter.get_or_insert_with(|| wgpu::util::TextureBlitter::new(&self.gpu_ctx.device, self.surface_format.unwrap()));

        blitter.copy(&self.gpu_ctx.device, &mut encoder, &vello_view, &surface_view);

        self.gpu_ctx.queue.submit(Some(encoder.finish()));
        self.gpu_ctx.queue.present(texture);

        let _ = self.viewport.dispatch_event_queue(&mut state, handle);
    }
}

pub(crate) fn get_client_size(hwnd: HWND) -> (u32, u32) {
    let mut rect = windows::Win32::Foundation::RECT::default();

    let res = unsafe { windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &raw mut rect) };

    if res.is_err() {
        return (0, 0);
    };

    ((rect.right - rect.left) as u32, (rect.bottom - rect.top) as u32)
}
