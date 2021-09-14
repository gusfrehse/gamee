use anyhow::*;
use std::time;
use wgpu::{util::DeviceExt, VertexBufferLayout};
use winit::{event::*, event_loop::ControlFlow, window::Window};

use crate::camera;
use crate::mesh;
use crate::texture;

const VERTICES_A: &[[f32; 3]] = &[
    [-0.5, 0.5, 0.0],
    [-0.5, -0.5, 0.0],
    [0.5, -0.5, 0.0],
    [0.5, 0.5, 0.0],
];

const NORMALS_A: &[[f32; 3]] = &[
    [0.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
];

const UVS_A: &[[f32; 2]] = &[
    [0.4131759, 0.00759614],
    [0.0048659444, 0.43041354],
    [0.28081453, 0.949397],
    [0.85967, 0.84732914],
];

const INDICES_A: &[u32] = &[0, 1, 2, 0, 2, 3];

pub struct State {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_cfg: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub clear_color: wgpu::Color,
    pub render_pipeline: wgpu::RenderPipeline,
    pub camera: camera::Camera,
    pub projection: camera::Projection,
    pub camera_controller: camera::Controller,
    pub camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    pub frame_count: u64,
    pub mesh: mesh::Mesh,
    pub depth_texture: texture::Texture,
    pub delta_time: time::Duration,
    pub last_frame_time: time::Instant,
    pub start_time: time::Instant,
    pub mouse_pressed: bool,
}

impl State {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_cfg = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        surface.configure(&device, &surface_cfg);

        let diffuse_bytes = include_bytes!("cool.png");

        let diffuse_texture =
            texture::Texture::from_bytes(&device, &queue, diffuse_bytes, Some("Cool texture"))
                .unwrap();

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &surface_cfg, "depth_texture");

        //let mesh_descriptor = mesh::Descriptor {
        //    vertices: VERTICES_A.to_vec(),
        //    normals: NORMALS_A.to_vec(),
        //    uvs: UVS_A.to_vec(),
        //    triangles: INDICES_A.to_vec(),
        //    texture: diffuse_texture,
        //};

        let perlin_bytes = include_bytes!("cool.png");
        let perlin_image = image::load_from_memory(perlin_bytes).unwrap();
        let mut mesh_descriptor =
            mesh::Descriptor::from_height_map(&perlin_image, 200, 200, 0.5, &device, &queue);

        let mesh = mesh_descriptor.bake(&device);

        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };

        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let camera = camera::Camera::new((0.0, 5.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));
        let projection = camera::Projection::new(
            surface_cfg.width,
            surface_cfg.height,
            cgmath::Deg(90.0),
            0.1,
            10000.0,
        );
        let camera_controller = camera::Controller::new(10.0, 0.3);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera, &projection);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        has_dynamic_offset: false,
                        min_binding_size: None,
                        ty: wgpu::BufferBindingType::Uniform,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("Camera bind group"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&mesh.texture_layout, &camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "main",
                buffers: &[VertexBufferLayout {
                    array_stride: std::mem::size_of::<mesh::Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3, // Position
                        1 => Float32x3, // Normal
                        2 => Float32x2  // UV
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: surface_cfg.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::all(),
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });

        Self {
            surface,
            device,
            queue,
            surface_cfg,
            size,
            clear_color,
            render_pipeline,
            camera,
            projection,
            camera_controller,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            frame_count: 0,
            mesh,
            depth_texture,
            delta_time: time::Duration::from_millis(13),
            last_frame_time: time::Instant::now(),
            start_time: time::Instant::now(),
            mouse_pressed: false,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.projection.resize(new_size.width, new_size.height);
            self.size = new_size;
            self.surface_cfg.width = new_size.width;
            self.surface_cfg.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_cfg);
            self.depth_texture = texture::Texture::create_depth_texture(
                &self.device,
                &self.surface_cfg,
                "depth_texture",
            );
        }
    }

    pub fn input<T>(&mut self, event: &Event<T>) -> ControlFlow {
        use winit::event::ElementState;
        match event {
            Event::WindowEvent {
                ref event,
                window_id: _,
            } => match event {
                WindowEvent::Resized(physical_size) => {
                    self.resize(*physical_size);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    self.resize(**new_inner_size);
                }
                WindowEvent::CloseRequested => return ControlFlow::Exit,
                _ => {}
            },
            Event::DeviceEvent {
                // Do not use device events for key board input, use window events
                ref event,
                device_id: _,
            } => match event {
                DeviceEvent::Key(KeyboardInput {
                    virtual_keycode: Some(keycode),
                    state,
                    ..
                }) => return self.keyboard_input(keycode, state),

                DeviceEvent::MouseWheel { delta, .. } => {
                    self.camera_controller.process_scroll(delta);
                }
                DeviceEvent::Button { button: 1, state } => {
                    self.mouse_pressed = *state == ElementState::Pressed;
                }
                DeviceEvent::MouseMotion { delta } => {
                    self.camera_controller.process_mouse(delta.0, delta.1);
                }
                _ => {}
            },
            _ => {}
        };

        ControlFlow::Poll
    }

    fn keyboard_input(&mut self, key: &VirtualKeyCode, state: &ElementState) -> ControlFlow {
        self.camera_controller.process_keyboard(*key, *state);

        if *key == VirtualKeyCode::Escape {
            ControlFlow::Exit
        } else {
            ControlFlow::Poll
        }
    }

    pub fn update(&mut self) {
        self.delta_time = self.last_frame_time.elapsed();
        self.last_frame_time = time::Instant::now();
        //println!(
        //    "\r{},{}",
        //    self.start_time.elapsed().as_secs_f32(),
        //    self.delta_time.as_secs_f32(),
        //);
        self.camera_controller
            .update_camera(&mut self.camera, self.delta_time);
        self.camera_uniform
            .update_view_proj(&self.camera, &self.projection);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
        self.frame_count += 1;
    }

    pub fn render(&mut self) -> Result<()> {
        mesh::Mesh::draw(&self)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_pos: [f32; 4],
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_pos: [0.0; 4],
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &camera::Camera, projection: &camera::Projection) {
        self.view_pos = camera.pos.to_homogeneous().into();
        self.view_proj = (projection.proj_mat() * camera.view_mat()).into();
    }
}
