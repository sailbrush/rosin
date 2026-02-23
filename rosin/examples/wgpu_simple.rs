#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{borrow::Cow, sync::RwLock};

use rosin::{prelude::*, wgpu, widgets::*};

static SHADER: &str = r#"@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(in_vertex_index) - 1);
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1);
    return vec4<f32>(x, y, 0.0, 1.0);
}
@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 0.0, 1.0);
}"#;

struct State {
    style: Stylesheet,
    pipeline: RwLock<Option<wgpu::RenderPipeline>>,
    count: Var<i32>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            style: stylesheet!("examples/styles/wgpu.css"),
            pipeline: RwLock::new(None),
            count: Var::new(0),
        }
    }
}

fn main_view(state: &State, ui: &mut Ui<State, WindowHandle>) {
    ui.node().id(id!()).style_sheet(&state.style).classes("root").children(|ui| {
        label(ui, id!(), *state.count).classes("number");
        button(ui, id!(), "Count", |s, _| {
            *s.count.write() += 1;
        });
    });
}

fn wgpu_callback(state: &State, ctx: &mut WgpuCtx<'_>) {
    let Ok(mut pipeline_guard) = state.pipeline.write() else { return };
    let pipeline = pipeline_guard.get_or_insert_with(|| {
        let shader = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
        });

        let pipeline_layout = ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        ctx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
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
                targets: &[Some(ctx.target_format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
    });

    let mut render_pass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: ctx.target,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    render_pass.set_pipeline(pipeline);
    render_pass.draw(0..3, 0..1);
}

#[rustfmt::skip]
fn main() {
    env_logger::init();

    let window = WindowDesc::new(callback!(main_view))
        .wgpu(callback!(wgpu_callback))
        .title("WGPU Example")
        .size(400, 300)
        .min_size(250, 150);

    AppLauncher::new(window)
        .run(State::default(), TranslationMap::default())
        .expect("Failed to launch");
}
