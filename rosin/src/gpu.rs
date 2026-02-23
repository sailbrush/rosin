//! Types for configuring and interacting with wgpu.

use std::cell::RefCell;

use crate::wgpu;

pub(crate) const COMPOSITE_SHADER: &str = r#"
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

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dim = textureDimensions(t_diffuse);
    let uv = pos.xy / vec2<f32>(f32(dim.x), f32(dim.y));
    return textureSample(t_diffuse, s_diffuse, uv);
}
"#;

/// Configuration for the underlying `wgpu` rendering backend.
///
/// This allows fine-tuning of hardware selection and GPU capabilities.
pub struct WgpuConfig {
    /// The preference for power usage. Defaults to `HighPerformance`.
    pub power_preference: wgpu::PowerPreference,
    /// Hardware features required by the application.
    pub features: wgpu::Features,
    /// Resource usage limits.
    pub limits: wgpu::Limits,
    /// Hints for memory allocation strategies.
    pub memory_hints: wgpu::MemoryHints,
    /// Which graphics APIs to enable.
    pub backends: wgpu::Backends,
}

impl Default for WgpuConfig {
    fn default() -> Self {
        Self {
            power_preference: wgpu::PowerPreference::HighPerformance,
            features: wgpu::Features::default(),
            limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            backends: wgpu::Backends::all(),
        }
    }
}

/// A collection of the types needed to render with wgpu.
///
/// This is provided to the wgpu callback associated with a window, if any.
pub struct WgpuCtx<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub target: &'a wgpu::TextureView,
    pub target_format: wgpu::TextureFormat,
    pub encoder: &'a mut wgpu::CommandEncoder,
}

pub(crate) struct OverlayPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub layout: wgpu::BindGroupLayout,
    pub sampler: wgpu::Sampler,
}

pub(crate) struct Compositor {
    /// Used when no wgpu callback has been provided.
    pub blitter: RefCell<Option<wgpu::util::TextureBlitter>>,
    /// Used when a wgpu callback has been provided,
    /// allowing the UI to be blended over a custom rendered background.
    pub custom: RefCell<Option<OverlayPipeline>>,
}

pub(crate) struct GpuCtx {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub compositor: Compositor,
}
