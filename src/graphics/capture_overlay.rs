use crate::export::png::CapturedPng;
use crate::{Color, ShapeInstance};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct PreviewUniforms {
    rect: [f32; 4],
    opacity: f32,
    _pad: [f32; 3],
}

pub enum CaptureOverlayParams<'a> {
    None,
    Flash {
        progress: f32,
    },
    Preview {
        image: &'a CapturedPng,
        backdrop_alpha: f32,
        flash_alpha: f32,
        rect: [f32; 4],
        opacity: f32,
    },
}

pub struct CaptureOverlay {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform_buffer: wgpu::Buffer,
    texture_format: wgpu::TextureFormat,
    texture: Option<wgpu::Texture>,
    _texture_view: Option<wgpu::TextureView>,
    bind_group: Option<wgpu::BindGroup>,
    cached_size: (u32, u32),
}

impl CaptureOverlay {
    #[must_use]
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Capture Preview Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("preview.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Capture Preview Uniform Buffer"),
            size: size_of::<PreviewUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Capture Preview Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Capture Preview BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Capture PreviewLayout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Capture Preview Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let texture_format = if surface_format.is_srgb() {
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Rgba8Unorm
        };

        Self {
            pipeline,
            bind_group_layout,
            sampler,
            uniform_buffer,
            texture_format,
            texture: None,
            _texture_view: None,
            bind_group: None,
            cached_size: (0, 0),
        }
    }

    pub fn ensure_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &CapturedPng,
    ) {
        if self.bind_group.is_some() && self.cached_size == (image.width, image.height) {
            return;
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Capture Preview Texture"),
            size: wgpu::Extent3d {
                width: image.width,
                height: image.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.texture_format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &image.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * image.width),
                rows_per_image: Some(image.height),
            },
            wgpu::Extent3d {
                width: image.width,
                height: image.height,
                depth_or_array_layers: 1,
            },
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Capture Preview Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        self.texture = Some(texture);
        self._texture_view = Some(texture_view);
        self.bind_group = Some(bind_group);
        self.cached_size = (image.width, image.height);
    }

    pub fn render_image(
        &self,
        queue: &wgpu::Queue,
        pass: &mut wgpu::RenderPass,
        rect: [f32; 4],
        opacity: f32,
    ) {
        let Some(bind_group) = &self.bind_group else {
            return;
        };

        let uniforms = PreviewUniforms {
            rect,
            opacity,
            _pad: [0.0; 3],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..6, 0..1);
    }

    pub fn clear(&mut self) {
        self.texture = None;
        self._texture_view = None;
        self.bind_group = None;
        self.cached_size = (0, 0);
    }
}

pub fn flash_bars(progress: f32) -> Vec<ShapeInstance> {
    let t = progress.clamp(0.0, 1.0);
    if t < 0.005 {
        return Vec::new();
    }
    let white = Color::WHITE;
    let mut bars = Vec::with_capacity(4);
    bars.push(ShapeInstance::rect(-1.0, 1.0 - t, 2.0, t, white));
    bars.push(ShapeInstance::rect(-1.0, -1.0, 2.0, t, white));

    let mid_h = 2.0 - 2.0 * t;
    if mid_h < 0.05 {
        bars.push(ShapeInstance::rect(-1.0, -1.0 + t, t, mid_h, white));
        bars.push(ShapeInstance::rect(1.0 - t, -1.0 + t, t, mid_h, white));
    }
    bars
}

pub fn dark_backdrop(alpha: f32) -> Vec<ShapeInstance> {
    if alpha < 0.005 {
        return Vec::new();
    }
    vec![ShapeInstance::rect(
        -1.0,
        -1.0,
        2.0,
        2.0,
        Color::new(0.05, 0.05, 0.1, alpha),
    )]
}

pub fn photo_card_border(rect: [f32; 4], thickness: f32, opacity: f32) -> Vec<ShapeInstance> {
    if opacity < 0.005 || thickness < 0.001 {
        return Vec::new();
    }

    let [x0, y0, x1, y1] = rect;
    let t = thickness;
    let c = Color::new(1.0, 1.0, 1.0, opacity);
    vec![
        ShapeInstance::rect(x0 - t, y1, (x1 - x0) + 2.0 * t, t, c),
        ShapeInstance::rect(x0 - t, y0 - t, (x1 - x0) + 2.0 * t, t, c),
        ShapeInstance::rect(x0 - t, y0, t, y1 - y0, c),
        ShapeInstance::rect(x1, y0, t, y1 - y0, c),
    ]
}

pub fn compute_preview_rect(img_w: u32, img_h: u32, screen_w: u32, screen_h: u32) -> [f32; 4] {
    let sw = screen_w.max(1) as f32;
    let sh = screen_h.max(1) as f32;
    let iw = img_w.max(1) as f32;
    let ih = img_h.max(1) as f32;

    let max_w = sw * 0.82;
    let max_h = sh * 0.82;
    let scale = (max_w / iw).min(max_h / ih);

    let ndc_w = iw * scale / sw * 2.0;
    let ndc_h = ih * scale / sh * 2.0;

    [-ndc_w * 0.5, -ndc_h * 0.5, ndc_w * 0.5, ndc_h * 0.5]
}
