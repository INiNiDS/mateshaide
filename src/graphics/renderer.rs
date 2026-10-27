use crate::canvas::camera::{Camera, CameraUniform};
use crate::canvas::text::CanvasText;
use crate::core::color::Color;
use crate::graphics::capture_overlay::{self, CaptureOverlay, CaptureOverlayParams, GlowOverlay};
use crate::graphics::overlay::{OVERLAY_CAP, build_swatch_instances};
use crate::graphics::pipeline::RenderPipeline;
use crate::graphics::text_renderer::TextLayer;
use crate::shape::instance::ShapeInstance;
use crate::shape::registry::ShapeRegistry;
use std::sync::Arc;
use std::sync::mpsc;
use tracing::warn;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{CurrentSurfaceTexture, RenderPassColorAttachment, SurfaceColorSpace};
use winit::window::Window;

pub struct PendingReadback {
    buffer: wgpu::Buffer,
    pub width: u32,
    pub height: u32,
    padded_bpr: u32,
    swap_rb: bool,
    rx: mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>,
}

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
    text: TextLayer,
    overlay_text: TextLayer,
    overlay_bind_group: wgpu::BindGroup,
    overlay_instance_buffer: wgpu::Buffer,
    glow: GlowOverlay,
    capture_overlay: CaptureOverlay,
}

impl Renderer {
    pub async fn new(
        window: Arc<Window>,
        uniform: CameraUniform,
        shapes: &ShapeRegistry,
    ) -> Result<Self, RendererError> {
        let instance = wgpu::Instance::default();

        #[cfg(not(target_arch = "wasm32"))]
        let surface = instance
            .create_surface(Arc::clone(&window))
            .map_err(|e| RendererError::Surface(e.to_string()))?;

        #[cfg(target_arch = "wasm32")]
        let surface = {
            use winit::platform::web::WindowExtWebSys;
            let canvas = window
                .canvas()
                .ok_or_else(|| RendererError::Surface("window has no canvas".to_owned()))?;
            instance
                .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
                .map_err(|e| RendererError::Surface(e.to_string()))?
        };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|e| RendererError::NoAdapter(e.to_string()))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(|e| RendererError::Device(e.to_string()))?;

        let size = window.inner_size();
        let surf_caps = surface.get_capabilities(&adapter);

        #[cfg(target_arch = "wasm32")]
        let format = surf_caps.formats[0];

        #[cfg(not(target_arch = "wasm32"))]
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
            color_space: SurfaceColorSpace::default(),
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode: surf_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let camera_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera BGL"),
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
            label: Some("Camera BG"),
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

        let identity_uniform = {
            let m = glam::Mat4::IDENTITY;
            CameraUniform {
                view_proj: m,
                inverse_view_proj: m,
                params: glam::Vec4::ZERO,
            }
        };
        let overlay_camera_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Overlay Camera"),
            contents: bytemuck::cast_slice(&[identity_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let overlay_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Overlay BG"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: overlay_camera_buffer.as_entire_binding(),
            }],
        });
        let overlay_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Overlay Instance"),
            size: (OVERLAY_CAP * size_of::<ShapeInstance>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let text = TextLayer::new(&device, &queue, config.format);
        let overlay_text = TextLayer::new(&device, &queue, config.format);
        let glow = GlowOverlay::new(&device, config.format);
        let capture_overlay = CaptureOverlay::new(&device, config.format);

        Ok(Self {
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
            text,
            overlay_text,
            overlay_bind_group,
            overlay_instance_buffer,
            glow,
            capture_overlay,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn validate_wgsl(&self, wgsl: &str) -> Result<(), wgpu::Error> {
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let _ = self
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
        warn!("Recreating surface");
        #[cfg(not(target_arch = "wasm32"))]
        let ns = self
            .instance
            .create_surface(Arc::clone(&self.window))
            .unwrap();
        #[cfg(target_arch = "wasm32")]
        let ns = {
            use winit::platform::web::WindowExtWebSys;
            let c = self.window.canvas().unwrap();
            self.instance
                .create_surface(wgpu::SurfaceTarget::Canvas(c))
                .unwrap()
        };
        ns.configure(&self.device, &self.config);
        self.surface = ns;
    }

    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }

    #[allow(clippy::too_many_arguments)]
    // Oops HIGH COMPLEXITY TODO
    pub fn redraw(
        &mut self,
        camera: &Camera,
        instances: &[ShapeInstance],
        texts: &[CanvasText],
        shapes: &ShapeRegistry,
        swatch: Option<Color>,
        capture: CaptureOverlayParams<'_>,
        overlay_texts: &[CanvasText],
    ) {
        self.update_pipeline(shapes);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera.uniform()]),
        );

        if instances.len() > self.instance_capacity {
            self.instance_capacity = instances.len().next_power_of_two();
            self.instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Instance Resize"),
                size: (self.instance_capacity * size_of::<ShapeInstance>()) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !instances.is_empty() {
            self.queue
                .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instances));
        }

        self.text.prepare(&self.device, &self.queue, camera, texts);

        let output = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(t) => t,
            CurrentSurfaceTexture::Suboptimal(t) => {
                self.resize(self.size);
                t
            }
            CurrentSurfaceTexture::Timeout => {
                warn!("surface timeout");
                return;
            }
            CurrentSurfaceTexture::Occluded => return,
            CurrentSurfaceTexture::Outdated => {
                self.resize(self.size);
                return;
            }
            CurrentSurfaceTexture::Lost => {
                self.recreate_surface();
                return;
            }
            CurrentSurfaceTexture::Validation => {
                return;
            }
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main"),
                color_attachments: &[Some(Self::clear(&view))],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.render_pipeline.draw(
                &mut pass,
                &self.camera_bind_group,
                &self.instance_buffer,
                instances.len() as u32,
            );
            self.text.render(&mut pass);
        }

        if let Some(color) = swatch {
            let overlay = build_swatch_instances(color, self.size.width, self.size.height);
            self.queue.write_buffer(
                &self.overlay_instance_buffer,
                0,
                bytemuck::cast_slice(&overlay),
            );
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Swatch"),
                color_attachments: &[Some(Self::load(&view))],
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
            );
        }

        match capture {
            CaptureOverlayParams::None => {
                self.capture_overlay.clear();
            }

            CaptureOverlayParams::Converge { progress } => {
                let intensity = (0.4 + 0.6 * progress * progress).min(1.0);
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Glow"),
                    color_attachments: &[Some(Self::load(&view))],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                self.glow
                    .render(&self.queue, &mut pass, progress, intensity);
            }

            CaptureOverlayParams::Preview {
                image,
                flash_alpha,
                layout,
                anim,
                buttons,
                buttons_opacity,
            } => {
                self.capture_overlay
                    .ensure_texture(&self.device, &self.queue, image);

                let mut shapes = capture_overlay::button_shapes(&buttons, buttons_opacity);
                if flash_alpha > 0.001 {
                    shapes.push(ShapeInstance::rect(
                        -1.0,
                        -1.0,
                        2.0,
                        2.0,
                        Color::new(1.0, 1.0, 1.0, flash_alpha),
                    ));
                }

                if !shapes.is_empty() {
                    self.queue.write_buffer(
                        &self.overlay_instance_buffer,
                        0,
                        bytemuck::cast_slice(&shapes),
                    );
                }
                if !overlay_texts.is_empty() {
                    self.overlay_text
                        .prepare(&self.device, &self.queue, camera, overlay_texts);
                }

                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Preview"),
                    color_attachments: &[Some(Self::load(&view))],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

                self.capture_overlay.render_image(
                    &self.queue,
                    &mut pass,
                    (self.size.width, self.size.height),
                    &layout,
                    &anim,
                );
                if !shapes.is_empty() {
                    self.render_pipeline.draw(
                        &mut pass,
                        &self.overlay_bind_group,
                        &self.overlay_instance_buffer,
                        shapes.len() as u32,
                    )
                }
                if !overlay_texts.is_empty() {
                    self.overlay_text.render(&mut pass);
                }
            }
        }

        self.queue.submit(Some(encoder.finish()));
        self.queue.present(output);
        self.text.trim();
        self.overlay_text.trim();
    }

    pub fn start_export(
        &mut self,
        camera: &Camera,
        instances: &[ShapeInstance],
        texts: &[CanvasText],
        shapes: &ShapeRegistry,
        width: u32,
        height: u32,
    ) -> Result<PendingReadback, RendererError> {
        let max_dim = self.device.limits().max_texture_dimension_2d;
        if width == 0 || height == 0 || width > max_dim || height > max_dim {
            return Err(RendererError::Device(format!(
                "Invalid export size {width}x{height}, max {max_dim}"
            )));
        }
        self.update_pipeline(shapes);

        let cam_buf = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Export Cam"),
            contents: bytemuck::cast_slice(&[camera.uniform()]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let cam_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Export Cam BG"),
            layout: &self.camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: cam_buf.as_entire_binding(),
            }],
        });

        let inst_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Export Inst"),
            size: (instances.len().max(1) * size_of::<ShapeInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !instances.is_empty() {
            self.queue
                .write_buffer(&inst_buf, 0, bytemuck::cast_slice(instances));
        }

        self.text.prepare(&self.device, &self.queue, camera, texts);

        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Export Target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let tview = target.create_view(&wgpu::TextureViewDescriptor::default());

        let unpadded = width * 4;
        let padded = unpadded.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Export RB"),
            size: (padded * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Export Enc"),
            });
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Export Pass"),
                color_attachments: &[Some(Self::clear(&tview))],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.render_pipeline
                .draw(&mut pass, &cam_bg, &inst_buf, instances.len() as u32);
            self.text.render(&mut pass);
        }
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(enc.finish()));

        let (tx, rx) = mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });

        Ok(PendingReadback {
            buffer: readback,
            width,
            height,
            padded_bpr: padded,
            swap_rb: matches!(
                self.config.format,
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
            ),
            rx,
        })
    }

    pub fn try_finish_export(
        &self,
        pending: &PendingReadback,
    ) -> Option<Result<Vec<u8>, RendererError>> {
        if let Err(e) = self.device.poll(wgpu::PollType::Poll) {
            return Some(Err(RendererError::Export(format!(
                "device poll failed: {e}"
            ))));
        }

        match pending.rx.try_recv() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                return Some(Err(RendererError::Export(e.to_string())));
            }
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                return Some(Err(RendererError::Export(
                    "readback callback was dropped".to_owned(),
                )));
            }
        }

        let unpadded_bpr = pending.width * 4;
        let mut pixels = Vec::with_capacity((unpadded_bpr * pending.height) as usize);
        {
            let data = match pending.buffer.slice(..).get_mapped_range() {
                Ok(data) => data,
                Err(e) => {
                    return Some(Err(RendererError::Export(format!(
                        "get_mapped_range failed: {e}"
                    ))));
                }
            };
            for row in 0..pending.height as usize {
                let start = row * pending.padded_bpr as usize;
                pixels.extend_from_slice(&data[start..start + unpadded_bpr as usize]);
            }
        }
        pending.buffer.unmap();

        if pending.swap_rb {
            for px in pixels.chunks_exact_mut(4) {
                px.swap(0, 2);
            }
        }

        Some(Ok(pixels))
    }

    fn update_pipeline(&mut self, shapes: &ShapeRegistry) {
        if self.pipeline_version != shapes.version() {
            self.render_pipeline = RenderPipeline::new(
                &self.device,
                &self.config,
                &self.camera_bind_group_layout,
                &shapes.build_wgsl(),
            );
            self.pipeline_version = shapes.version();
        }
    }

    fn clear(view: &wgpu::TextureView) -> RenderPassColorAttachment<'_> {
        RenderPassColorAttachment {
            view,
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
        }
    }

    fn load(view: &wgpu::TextureView) -> RenderPassColorAttachment<'_> {
        RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error("failed to create surface: {0}")]
    Surface(String),
    #[error("no WebGPU adapter: {0}")]
    NoAdapter(String),
    #[error("failed to create device: {0}")]
    Device(String),
    #[error("export failed: {0}")]
    Export(String),
}
