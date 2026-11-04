use crate::plugins::{UiImage, UiImageKind};
use anyhow::{Result, ensure};
use wgpu::PipelineCompilationOptions;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ImageQuad {
    rect: [f32; 4],
    uv_rect: [f32; 4],
}

struct ImageTexture {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

pub(crate) struct UiImageRenderer {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    textures: [Option<ImageTexture>; UiImageKind::COUNT],
    loaded: bool,
    texture_format: wgpu::TextureFormat,
    buffer: wgpu::Buffer,
    capacity: usize,
    count: usize,
}

impl UiImageRenderer {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UI images shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ui_images.wgsl").into()),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("UI images layout"),
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

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("UI images sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("UI images pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("UI images pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<ImageQuad>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x4,
                        1 => Float32x4
                    ],
                })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let capacity = 8;
        let buffer = Self::create_buffer(device, capacity);

        Self {
            pipeline,
            layout,
            sampler,
            textures: std::array::from_fn(|_| None),
            loaded: false,
            texture_format: if format.is_srgb() {
                wgpu::TextureFormat::Rgba8UnormSrgb
            } else {
                wgpu::TextureFormat::Rgba8Unorm
            },
            buffer,
            capacity,
            count: 0,
        }
    }

    fn create_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI image quads"),
            size: (capacity * size_of::<ImageQuad>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn load_texture(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
    ) -> Result<ImageTexture> {
        let mut reader =
            image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);

        let max_dimension = device.limits().max_texture_dimension_2d.min(2048);

        let mut limits = image::Limits::default();
        limits.max_image_width = Some(max_dimension);
        limits.max_image_height = Some(max_dimension);
        limits.max_alloc = Some(32 * 1024 * 1024);
        reader.limits(limits);

        let decoded = reader.decode()?;
        let remove_white_matte = !decoded.color().has_alpha();
        let mut rgba = decoded.to_rgba8();

        if remove_white_matte {
            for pixel in rgba.pixels_mut() {
                let white = pixel[0].min(pixel[1]).min(pixel[2]);
                let alpha = 255 - white;

                if alpha == 0 {
                    pixel.0 = [0, 0, 0, 0];
                    continue;
                }

                for channel in &mut pixel.0[..3] {
                    *channel = ((u32::from(*channel - white) * 255 + u32::from(alpha) / 2)
                        / u32::from(alpha)) as u8;
                }

                pixel[3] = alpha;
            }
        }

        let (width, height) = rgba.dimensions();

        ensure!(width > 0 && height > 0, "empty PNG");

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Halloween PNG"),
            size,
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
            rgba.as_raw(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Halloween image bind group"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        Ok(ImageTexture {
            _texture: texture,
            _view: view,
            bind_group,
            width,
            height,
        })
    }

    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen: [f32; 2],
        images: &[UiImage],
    ) {
        self.count = images.len();

        if images.is_empty() {
            return;
        }

        if !self.loaded {
            let sources: [(&str, &[u8]); UiImageKind::COUNT] = [
                ("blood.png", include_bytes!("../../assets/blood.png")),
                ("pumpkin.png", include_bytes!("../../assets/pumpkin.png")),
                ("strips.png", include_bytes!("../../assets/strips.png")),
                ("web.png", include_bytes!("../../assets/web.png")),
                ("mascot.png", include_bytes!("../../assets/mascot.png")),
                ("web2.png", include_bytes!("../../assets/web2.png")),
                ("eyes.png", include_bytes!("../../assets/eyes.png")),
                ("spider.png", include_bytes!("../../assets/spider.png")),
            ];

            for (index, (name, bytes)) in sources.into_iter().enumerate() {
                match self.load_texture(device, queue, bytes) {
                    Ok(texture) => {
                        self.textures[index] = Some(texture);
                    }

                    Err(error) => {
                        tracing::warn!("Could not load {name}: {error:#}");
                    }
                }
            }

            self.loaded = true;
        }

        if images.len() > self.capacity {
            self.capacity = images.len().next_power_of_two();
            self.buffer = Self::create_buffer(device, self.capacity);
        }

        let [w, h] = [screen[0].max(1.0), screen[1].max(1.0)];

        let vertices: Vec<ImageQuad> = images
            .iter()
            .map(|image| {
                let [left, top, right, bottom] = image.bounds;

                let bw = (right - left).max(0.0);
                let bh = (bottom - top).max(0.0);

                let aspect = self.textures[image.kind as usize]
                    .as_ref()
                    .map_or(1.0, |texture| texture.width as f32 / texture.height as f32);

                let width = bw.min(bh * aspect);
                let height = width / aspect;

                let x = left + (bw - width) * image.anchor[0];
                let y = top + (bh - height) * image.anchor[1];

                ImageQuad {
                    rect: [
                        2.0 * x / w - 1.0,
                        1.0 - 2.0 * y / h,
                        2.0 * (x + width) / w - 1.0,
                        1.0 - 2.0 * (y + height) / h,
                    ],
                    uv_rect: if image.flip_x {
                        [1.0, 0.0, 0.0, 1.0]
                    } else {
                        [0.0, 0.0, 1.0, 1.0]
                    },
                }
            })
            .collect();

        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&vertices));
    }

    pub(crate) fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        index: usize,
        kind: UiImageKind,
    ) {
        if index >= self.count {
            return;
        }

        let Some(texture) = &self.textures[kind as usize] else {
            return;
        };

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &texture.bind_group, &[]);
        pass.set_vertex_buffer(0, self.buffer.slice(..));

        pass.draw(0..6, index as u32..index as u32 + 1);
    }
}
