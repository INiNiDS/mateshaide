use crate::camera::CameraUniform;
use crate::render_pipeline::RenderPipeline;
use crate::shape::{ShapeInstance, ShapeRegistry};
use std::sync::Arc;
use tracing::warn;
use wgpu::CurrentSurfaceTexture;
use wgpu::util::DeviceExt;
use winit::window::Window;
use crate::color::Color;
use crate::palette;

const SWATCH_PX: f32 = 48.0;
const OVERLAY_CAP: usize = 8; // For test (will remove)
const SWATCH_BORDER_PX: f32 = 2.0;
const SWATCH_MARGIN_PX: f32 = 16.0;

pub struct Renderer {
    window: Arc<Window>,
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: RenderPipeline,
    pipeline_version: u64,
    size: winit::dpi::PhysicalSize<u32>,
    camera_buffer: wgpu::Buffer,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    camera_bind_group: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,

    // Test (will remove)
    overlay_bind_group: wgpu::BindGroup,
    overlay_instance_buffer: wgpu::Buffer,
}

impl Renderer {
    pub async fn new(window: Arc<Window>, uniform: CameraUniform, shapes: &ShapeRegistry) -> Self {
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("Failed to create device");

        let size = window.inner_size();
        let surf_caps = surface.get_capabilities(&adapter);

        let format = surf_caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(surf_caps.formats[0]);

        let present_mode = if surf_caps
            .present_modes
            .contains(&wgpu::PresentMode::Mailbox)
        {
            wgpu::PresentMode::Mailbox
        } else {
            wgpu::PresentMode::Fifo
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: Default::default(),
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode: surf_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let initial_capacity = 64;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: (initial_capacity * size_of::<ShapeInstance>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let render_pipeline = RenderPipeline::new(
            &device,
            &config,
            &camera_bind_group_layout,
            &shapes.build_wgsl(),
        );

        let indetity_uniform = {
            let m = glam::Mat4::IDENTITY;
            CameraUniform {
                view_proj: m,
                inverse_view_proj: m,
                params: glam::Vec4::ZERO,
            }
        };
        let overlay_camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Overlay Camera Buffer"),
            contents: bytemuck::cast_slice(&[indetity_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let overlay_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Overlay Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: overlay_camera_buffer.as_entire_binding(),
            }],
        });
        let overlay_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Overlay Instance Buffer"),
            size: (OVERLAY_CAP * size_of::<ShapeInstance>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            instance,
            surface,
            device,
            queue,
            config,
            window,
            size,
            render_pipeline,
            pipeline_version: shapes.version(),
            camera_buffer,
            camera_bind_group_layout,
            camera_bind_group,
            instance_buffer,
            instance_capacity: initial_capacity,
            overlay_bind_group,
            overlay_instance_buffer,
        }
    }

    pub fn validate_wgsl(&self, wgsl: &str) -> Result<(), wgpu::Error> {
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let _module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Validation"),
                source: wgpu::ShaderSource::Wgsl(wgsl.into()),
            });
        match pollster::block_on(scope.pop()) {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn recreate_surface(&mut self) {
        warn!("Recreating surface due to lost surface texture");

        let new_surface = self
            .instance
            .create_surface(self.window.clone())
            .expect("Failed to recreate surface");

        new_surface.configure(&self.device, &self.config);
        self.surface = new_surface;
    }

    pub fn redraw(
        &mut self,
        uniform: CameraUniform,
        instances: &[ShapeInstance],
        shapes: &ShapeRegistry,
        swatch: Option<Color>
    ) {
        if self.pipeline_version != shapes.version() {
            self.render_pipeline = RenderPipeline::new(
                &self.device,
                &self.config,
                &self.camera_bind_group_layout,
                &shapes.build_wgsl(),
            );
            self.pipeline_version = shapes.version();
        }

        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[uniform]));

        if instances.len() > self.instance_capacity {
            self.instance_capacity = instances.len().next_power_of_two();
            self.instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Instance Buffer Resized"),
                size: (self.instance_capacity * size_of::<ShapeInstance>()) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        if !instances.is_empty() {
            self.queue
                .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instances));
        }

        let output = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(texture) => texture,
            CurrentSurfaceTexture::Suboptimal(texture) => {
                self.resize(self.size);
                texture
            }
            CurrentSurfaceTexture::Timeout => {
                warn!("Timeout while acquiring next surface texture");
                return;
            }
            CurrentSurfaceTexture::Occluded => return,
            CurrentSurfaceTexture::Outdated => {
                self.resize(self.size);
                return;
            }
            CurrentSurfaceTexture::Lost => {
                warn!("Surface texture is lost");
                self.recreate_surface();
                return;
            }
            CurrentSurfaceTexture::Validation => {
                panic!("Validation error");
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.94,
                            g: 0.945,
                            b: 0.955,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            self.render_pipeline.draw(
                &mut render_pass,
                &self.camera_bind_group,
                &self.instance_buffer,
                instances.len() as u32,
            );
        }

        if let Some(color) = swatch {
            let overlay = build_swatch_instances(color, self.size.width, self.size.height);
            self.queue.write_buffer(
                &self.overlay_instance_buffer,
                0,
                bytemuck::cast_slice(&overlay),
            );

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Overlay Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            self.render_pipeline.draw(
                &mut pass,
                &self.overlay_bind_group,
                &self.overlay_instance_buffer,
                overlay.len() as u32,
            )
        }

        self.queue.submit(Some(encoder.finish()));
        self.queue.present(output);
    }
}

fn build_swatch_instances(color: Color, width: u32, height: u32) -> Vec<ShapeInstance> {
    let w = width.max(1) as f32;
    let h = height.max(1) as f32;

    let outer_half_px = SWATCH_PX * 0.5 + SWATCH_BORDER_PX;
    let inner_half_px = SWATCH_PX * 0.5;

    let ohx = 2.0 * outer_half_px / w;
    let ohy = 2.0 * outer_half_px / h;
    let ihx = 2.0 * inner_half_px / w;
    let ihy = 2.0 * inner_half_px / h;

    let cx_px = w - SWATCH_MARGIN_PX - outer_half_px;
    let cy_px =  SWATCH_MARGIN_PX + outer_half_px;

    let cx = 2.0 * cx_px / w - 1.0;
    let cy = 1.0 - 2.0 * cy_px / h;

    vec![
        ShapeInstance::rect(cx - ohx, cy - ohy, ohx * 2.0, ohy * 2.0, palette::GRAY_700),
        ShapeInstance::rect(cx - ihx, cy - ihy, ihx * 2.0, ihy * 2.0, color),
    ]
}