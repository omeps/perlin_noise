use rand::RngCore;
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::NamedKey,
    window::{Window, WindowBuilder},
};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    t: f32,
    s: f32,
}

struct State<'a> {
    surface: wgpu::Surface<'a>,
    compute_pipeline: wgpu::ComputePipeline,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: &'a Window,
    params: Params,
    noise_texture: wgpu::Texture,
}
impl<'a> State<'a> {
    async fn new(window: &'a Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_limits: wgpu::Limits::default(),
                    required_features: wgpu::Features::empty(),
                },
                None,
            )
            .await
            .unwrap();
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&device.create_bind_group_layout(
                        &wgpu::BindGroupLayoutDescriptor {
                            label: None,
                            entries: &[
                                wgpu::BindGroupLayoutEntry {
                                    binding: 0,
                                    visibility: wgpu::ShaderStages::COMPUTE,
                                    ty: wgpu::BindingType::Texture {
                                        sample_type: wgpu::TextureSampleType::Float {
                                            filterable: false,
                                        },
                                        multisampled: false,
                                        view_dimension: wgpu::TextureViewDimension::D2,
                                    },
                                    count: None,
                                },
                                wgpu::BindGroupLayoutEntry {
                                    binding: 1,
                                    visibility: wgpu::ShaderStages::COMPUTE,
                                    ty: wgpu::BindingType::StorageTexture {
                                        access: wgpu::StorageTextureAccess::WriteOnly,
                                        format: wgpu::TextureFormat::Rgba8Unorm,
                                        view_dimension: wgpu::TextureViewDimension::D2,
                                    },
                                    count: None,
                                },
                                wgpu::BindGroupLayoutEntry {
                                    binding: 2,
                                    visibility: wgpu::ShaderStages::COMPUTE,
                                    ty: wgpu::BindingType::Buffer {
                                        ty: wgpu::BufferBindingType::Uniform,
                                        has_dynamic_offset: false,
                                        min_binding_size: None,
                                    },
                                    count: None,
                                },
                            ],
                        },
                    )],
                    push_constant_ranges: &[],
                }),
            ),
            module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Perlin Noise"),
                source: wgpu::ShaderSource::Wgsl(include_str!("perlin_noise.wgsl").into()),
            }),
            entry_point: "draw",
        });
        let surface_caps = surface.get_capabilities(&adapter);
        let mut noise_data = [0; 256 * 256];
        rand::thread_rng().fill_bytes(&mut noise_data);
        let noise_texture = device.create_texture_with_data(
            &queue,
            &wgpu::TextureDescriptor {
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                label: Some("noise texture descriptor"),
                mip_level_count: 1,
                sample_count: 1,
                size: wgpu::Extent3d {
                    width: 256,
                    height: 256,
                    depth_or_array_layers: 1,
                },
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[wgpu::TextureFormat::R8Unorm],
            },
            wgpu::util::TextureDataOrder::default(),
            &noise_data,
        );
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::STORAGE_BINDING,
            format: wgpu::TextureFormat::Rgba8Unorm,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);
        Self {
            surface,
            window,
            compute_pipeline,
            device,
            queue,
            config,
            size,
            params: Params { t: 0.0, s: 250.0 },
            noise_texture,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {
        self.params.t += 0.00;
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view_window = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let view_noise = self
            .noise_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let param_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("param buffer"),
                    contents: bytemuck::cast_slice(&[self.params]),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
            let bind_group_layout = self.compute_pipeline.get_bind_group_layout(0);
            let bind_group_entry_noise = wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view_noise),
            };
            let bind_group_entry_window = wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view_window),
            };
            let bind_group_entry_param = wgpu::BindGroupEntry {
                binding: 2,
                resource: param_buffer.as_entire_binding(),
            };
            let bind_group_descriptor = wgpu::BindGroupDescriptor {
                label: Some("Bind Group Layout"),
                layout: &bind_group_layout,
                entries: &[
                    bind_group_entry_noise,
                    bind_group_entry_window,
                    bind_group_entry_param,
                ],
            };
            let bind_group = self.device.create_bind_group(&bind_group_descriptor);
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(self.size.width, self.size.height, 1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
pub async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut state = State::new(&window).await;

    event_loop
        .run(|event, control_flow| match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    logical_key: winit::keyboard::Key::Named(NamedKey::Escape),
                                    ..
                                },
                            ..
                        } => control_flow.exit(),
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { .. } => {
                            // new_inner_size is &&mut so we have to dereference it twice
                            state.resize(window.inner_size());
                        }
                        WindowEvent::RedrawRequested => {
                            state.update();
                            match state.render() {
                                Ok(_) => {}
                                // Reconfigure the surface if lost
                                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                                // The system is out of memory, we should probably quit
                                Err(wgpu::SurfaceError::OutOfMemory) => {
                                    control_flow.exit();
                                }
                                // All other errors (Outdated, Timeout) should be resolved by the next frame
                                Err(e) => eprintln!("{:?}", e),
                            }
                        }

                        _ => {}
                    }
                }
            }
            Event::AboutToWait => {
                // RedrawRequested will only trigger once unless we manually
                // request it.
                state.window().request_redraw();
            }
            _ => {}
        })
        .unwrap();
}
