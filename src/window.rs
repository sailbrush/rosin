use crate::layout::*;
use crate::render;
use crate::tree::*;
use crate::view::*;
use crate::{app::Stage, libloader::LibLoader, style::Stylesheet};

use std::{borrow::Cow, error::Error, mem, ptr::NonNull};

use bumpalo::collections::Vec as BumpVec;
use futures::executor::block_on;
use winit::{
    dpi::{LogicalSize, PhysicalSize},
    event_loop::EventLoopWindowTarget,
    window::{Window, WindowBuilder, WindowId},
};

pub struct WindowDesc<T> {
    pub(crate) builder: WindowBuilder,
    pub(crate) view: View<T>,
}

impl<T> WindowDesc<T> {
    pub fn new(view: View<T>) -> Self {
        Self {
            builder: WindowBuilder::new(),
            view,
        }
    }

    pub fn with_title<S: Into<String>>(mut self, title: S) -> Self {
        self.builder = self.builder.with_title(title);
        self
    }

    pub fn with_size(mut self, width: f64, height: f64) -> Self {
        self.builder = self.builder.with_inner_size(LogicalSize::new(width, height));
        self
    }

    //TODO wrap other WindowBuilder functions
}

// A struct with the same size as BumpVec used to erase lifetimes
// A BumpVec that has been orphaned from its allocator
struct OrphanVec {
    _a: NonNull<()>,
    _b: NonNull<()>,
    _c: NonNull<()>,
    _d: NonNull<()>,
}

impl OrphanVec {
    pub unsafe fn orphan<U>(vec: BumpVec<U>) -> OrphanVec {
        mem::transmute::<BumpVec<U>, OrphanVec>(vec)
    }

    pub unsafe fn adopt<U>(&self) -> &BumpVec<U> {
        &*(self as *const OrphanVec as *const BumpVec<U>)
    }

    pub unsafe fn adopt_mut<U>(&mut self) -> &mut BumpVec<U> {
        &mut *(self as *mut OrphanVec as *mut BumpVec<U>)
    }
}

pub(crate) struct RosinWindow<T> {
    window: Window,
    view: View<T>,
    stage: Stage,
    alloc: Alloc,
    tree_cache: Option<OrphanVec>,
    layout_cache: Option<OrphanVec>,

    surface: wgpu::Surface,
    device: wgpu::Device,
    sampler: wgpu::Sampler,
    queue: wgpu::Queue,
    texture: wgpu::Texture,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    swap_chain_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    render_pipeline: wgpu::RenderPipeline,
}

impl<T> RosinWindow<T> {
    pub fn new(desc: WindowDesc<T>, event_loop: &EventLoopWindowTarget<()>) -> Result<Self, Box<dyn Error>> {
        let window = desc.builder.build(event_loop)?;
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::BackendBit::all());
        let surface = unsafe { instance.create_surface(&window) };
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
        }))
        .expect("[Rosin] Failed to find an appropriate adapter");

        let (device, queue) = block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        ))
        .expect("[Rosin] Failed to create device");

        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("blit.wgsl"))),
            flags: wgpu::ShaderFlags::VALIDATION,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 1.0,
            compare: None,
            anisotropy_clamp: None,
            border_color: None,
        });

        let texture_extent = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        comparison: false,
                        filtering: false,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let swapchain_format = adapter.get_swap_chain_preferred_format(&surface);

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[swapchain_format.into()],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        let swap_chain_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        let swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);

        Ok(Self {
            window,
            view: desc.view,
            stage: Stage::Build,
            alloc: Alloc::default(),
            tree_cache: None,
            layout_cache: None,

            surface,
            device,
            sampler,
            queue,
            texture,
            bind_group_layout,
            bind_group,
            swap_chain_desc,
            swap_chain,
            render_pipeline,
        })
    }

    fn reset_cache(&mut self) {
        self.layout_cache = None;
        self.tree_cache = None;
        self.alloc.bump.reset();
    }

    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    pub fn borrow_tree_cache(&self) -> Option<&BumpVec<ArrayNode<T>>> {
        // SAFETY: The returned borrow is guaranteed to remain valid because
        // it points to heap allocated memory that requires &mut to clear
        Some(unsafe { self.tree_cache.as_ref()?.adopt() })
    }

    pub fn borrow_layout_cache(&self) -> Option<&BumpVec<Layout>> {
        // SAFETY: The returned borrow is guaranteed to remain valid because
        // it points to heap allocated memory that requires &mut to clear
        Some(unsafe { self.layout_cache.as_ref()?.adopt() })
    }

    pub fn set_stage(&mut self, stage: Stage) {
        self.stage.keep_max(stage);
        if stage != Stage::Idle {
            self.window.request_redraw();
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.set_stage(Stage::Layout);

            self.swap_chain_desc.width = new_size.width;
            self.swap_chain_desc.height = new_size.height;
            self.swap_chain = self.device.create_swap_chain(&self.surface, &self.swap_chain_desc);

            self.texture.destroy();
            let texture_extent = wgpu::Extent3d {
                width: new_size.width,
                height: new_size.height,
                depth: 1,
            };
            self.texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: texture_extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            });
            let texture_view = self.texture.create_view(&wgpu::TextureViewDescriptor::default());

            self.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
        }
    }

    pub fn redraw(
        &mut self,
        state: &T,
        stylesheet: &Stylesheet,
        loader: &Option<LibLoader>,
    ) -> Result<(), wgpu::SwapChainError> {
        // Rebuild window tree
        if self.stage == Stage::Build || self.tree_cache.is_none() {
            self.reset_cache();
            let mut tree = self.view.get(loader)(&self.alloc, &state).finish(&self.alloc).unwrap();
            stylesheet.style(&mut tree);
            // SAFETY: This is needed to store self references, which allows us to retain bump allocated data between redraws
            self.tree_cache = Some(unsafe { OrphanVec::orphan(tree) });
        }

        let tree: &BumpVec<ArrayNode<T>> = unsafe { self.tree_cache.as_ref().unwrap().adopt() };

        // Recalculate layout
        let size = self.window.inner_size();
        if self.stage >= Stage::Layout || self.layout_cache.is_none() {
            if self.layout_cache.is_none() {
                let new_layout: BumpVec<Layout> = BumpVec::with_capacity_in(tree.len(), &self.alloc.bump);
                // SAFETY: This is needed to store self references, which allows us to retain bump allocated data between redraws
                self.layout_cache = Some(unsafe { OrphanVec::orphan(new_layout) });
            }

            let layout = unsafe { self.layout_cache.as_mut().unwrap().adopt_mut() };

            layout.clear();
            for _ in 0..tree.len() {
                layout.push(Layout::default());
            }

            Layout::solve(&tree, (size.width as f32, size.height as f32), layout);
        }

        let layout: &BumpVec<Layout> = unsafe { self.layout_cache.as_ref().unwrap().adopt() };

        // Render
        let dt = render::render(tree, layout);

        // Blit to screen
        let frame = self.swap_chain.get_current_frame()?.output;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.draw(0..3, 0..1);
        drop(render_pass);

        self.queue.write_texture(
            wgpu::TextureCopyView {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            dt.get_data_u8(),
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: 4 * size.width,
                rows_per_image: size.height,
            },
            wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth: 1,
            },
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        // Cleanup
        self.stage = Stage::Idle;
        Ok(())
    }
}
