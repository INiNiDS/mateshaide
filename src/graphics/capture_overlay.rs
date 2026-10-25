use crate::core::Color;
use crate::export::png::CapturedPng;
use crate::shape::ShapeInstance;


#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GlowU { progress: f32, intensity: f32, _pad: [f32; 2] }

pub struct GlowOverlay {
    pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl GlowOverlay {
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Glow"),
            source: wgpu::ShaderSource::Wgsl(include_str!("glow.wgsl").into()),
        });
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Glow U"),
            size: size_of::<GlowU>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Glow BGL"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Glow BG"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Glow PL"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Glow Pipeline"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self { pipeline, uniform_buf, bind_group }
    }

    pub fn render(&self, queue: &wgpu::Queue, pass: &mut wgpu::RenderPass, progress: f32, intensity: f32) {
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(&[GlowU {
            progress, intensity, _pad: [0.0; 2],
        }]));
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PreviewU { rect: [f32; 4], opacity: f32, _pad: [f32; 3] }

pub struct CaptureOverlay {
    pipeline: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform_buf: wgpu::Buffer,
    tex_format: wgpu::TextureFormat,
    texture: Option<wgpu::Texture>,
    _view: Option<wgpu::TextureView>,
    bind_group: Option<wgpu::BindGroup>,
    cached: (u32, u32),
}

impl CaptureOverlay {
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Preview"),
            source: wgpu::ShaderSource::Wgsl(include_str!("preview.wgsl").into()),
        });
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Preview U"),
            size: size_of::<PreviewU>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Preview S"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Preview BGL"),
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
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Preview PL"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Preview Pipeline"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let tex_format = if format.is_srgb() {
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Rgba8Unorm
        };
        Self {
            pipeline, bgl, sampler, uniform_buf, tex_format,
            texture: None, _view: None, bind_group: None, cached: (0, 0),
        }
    }

    pub fn ensure_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, image: &CapturedPng) {
        if self.bind_group.is_some() && self.cached == (image.width, image.height) {
            return;
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Preview Tex"),
            size: wgpu::Extent3d {
                width: image.width,
                height: image.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.tex_format,
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
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Preview BG"),
            layout: &self.bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.uniform_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });
        self.texture = Some(texture);
        self._view = Some(view);
        self.bind_group = Some(bg);
        self.cached = (image.width, image.height);
    }

    pub fn render_image(&self, queue: &wgpu::Queue, pass: &mut wgpu::RenderPass, rect: [f32; 4], opacity: f32) {
        let Some(bg) = &self.bind_group else { return };
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(&[PreviewU {
            rect, opacity, _pad: [0.0; 3],
        }]));
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bg, &[]);
        pass.draw(0..6, 0..1);
    }

    pub fn clear(&mut self) {
        self.texture = None;
        self._view = None;
        self.bind_group = None;
        self.cached = (0, 0);
    }
}

#[derive(Clone, Copy)]
pub struct ButtonRect {
    pub ndc: [f32; 4],
    pub screen: [f32; 4],
    pub label: &'static str,
    pub color: Color,
}

pub enum ButtonAction { Save, Copy }

#[must_use]
pub fn compute_button_rects(photo_rect: [f32; 4], sw: u32, sh: u32) -> [ButtonRect; 2] {
    let sw = sw.max(1) as f32;
    let sh = sh.max(1) as f32;

    let photo_bottom = (1.0 - photo_rect[1]) * 0.5 * sh;

    let bw: f32 = 160.0;
    let bh: f32 = 46.0;
    let gap: f32 = 14.0;
    let margin: f32 = 22.0;

    let top = photo_bottom + margin;
    let bottom = top + bh;
    let cx = sw * 0.5;

    let s_l = cx - bw - gap * 0.5;
    let s_r = cx - gap * 0.5;
    let c_l = cx + gap * 0.5;
    let c_r = cx + bw + gap * 0.5;

    let nx = |x: f32| x / sw * 2.0 - 1.0;
    let ny = |y: f32| 1.0 - y / sh * 2.0;

    [
        ButtonRect {
            ndc: [nx(s_l), ny(bottom), nx(s_r), ny(top)],
            screen: [s_l, top, s_r, bottom],
            label: "Save",
            color: Color::new(0.12, 0.42, 0.92, 1.0),
        },
        ButtonRect {
            ndc: [nx(c_l), ny(bottom), nx(c_r), ny(top)],
            screen: [c_l, top, c_r, bottom],
            label: "Copy",
            color: Color::new(0.08, 0.62, 0.32, 1.0),
        },
    ]
}

#[must_use]
pub fn button_shapes(buttons: &[ButtonRect; 2], opacity: f32) -> Vec<ShapeInstance> {
    buttons.iter().map(|b| {
        let [x0, y0, x1, y1] = b.ndc;
        ShapeInstance::rect(x0, y0, x1 - x0, y1 - y0, b.color.with_alpha(opacity))
    }).collect()
}


#[must_use]
pub fn photo_card_border(rect: [f32; 4], t: f32, opacity: f32) -> Vec<ShapeInstance> {
    if opacity < 0.005 || t < 0.001 { return Vec::new(); }
    let [x0, y0, x1, y1] = rect;
    let c = Color::new(0.0, 0.0, 0.0, opacity);
    vec![
        ShapeInstance::rect(x0 - t, y1, (x1 - x0) + 2.0 * t, t, c),
        ShapeInstance::rect(x0 - t, y0 - t, (x1 - x0) + 2.0 * t, t, c),
        ShapeInstance::rect(x0 - t, y0, t, y1 - y0, c),
        ShapeInstance::rect(x1, y0, t, y1 - y0, c),
    ]
}

#[must_use]
pub fn compute_preview_rect(img_w: u32, img_h: u32, sw: u32, sh: u32) -> [f32; 4] {
    let sw = sw.max(1) as f32;
    let sh = sh.max(1) as f32;
    let iw = img_w.max(1) as f32;
    let ih = img_h.max(1) as f32;
    let scale = (sw * 0.78 / iw).min(sh * 0.68 / ih);
    let nw = iw * scale / sw * 2.0;
    let nh = ih * scale / sh * 2.0;
    [-nw * 0.5, -nh * 0.5, nw * 0.5, nh * 0.5]
}

pub enum CaptureOverlayParams<'a> {
    None,
    Converge { progress: f32 },
    Preview {
        image: &'a CapturedPng,
        flash_alpha: f32,
        rect: [f32; 4],
        opacity: f32,
        buttons: [ButtonRect; 2],
    },
}