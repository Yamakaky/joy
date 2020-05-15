use cgmath::Vector3;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

mod camera;
mod uniforms;

const MAX_INSTANCE_COUNT: u64 = 320 * 240;

struct GUI {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    uniforms: uniforms::Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    size: winit::dpi::PhysicalSize<u32>,
    camera: camera::Camera,
    irdata: Box<[u32]>,
}

impl GUI {
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        let surface = wgpu::Surface::create(window);

        let adapter = wgpu::Adapter::request(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            },
            wgpu::BackendBit::PRIMARY,
        )
        .await
        .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                extensions: wgpu::Extensions {
                    anisotropic_filtering: false,
                },
                limits: wgpu::Limits::default(),
            })
            .await;

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let camera = camera::Camera::new(&sc_desc);
        let mut uniforms = uniforms::Uniforms::new();
        uniforms.update_view_proj(&camera);
        let uniform_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(&[uniforms]),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        );

        #[rustfmt::skip]
        let vertex_data: &[f32] = &[
            // front
            -0.4, -0.4,  0.4,
            0.4, -0.4,  0.4,
            0.4,  0.4,  0.4,
            -0.4,  0.4,  0.4,
            // back
            -0.4, -0.4, -0.4,
            0.4, -0.4, -0.4,
            0.4,  0.4, -0.4,
            -0.4,  0.4, -0.4,
        ];
        let vertex_buffer = device
            .create_buffer_with_data(bytemuck::cast_slice(vertex_data), wgpu::BufferUsage::VERTEX);

        #[rustfmt::skip]
        let index_data: &[u16] = &[
            // front
            0, 1, 2,
            2, 3, 0,
            // right
            1, 5, 6,
            6, 2, 1,
            // back
            7, 6, 5,
            5, 4, 7,
            // left
            4, 0, 3,
            3, 7, 4,
            // bottom
            4, 5, 1,
            1, 0, 4,
            // top
            3, 2, 6,
            6, 7, 3,
        ];
        let index_buffer = device
            .create_buffer_with_data(bytemuck::cast_slice(index_data), wgpu::BufferUsage::INDEX);

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            size: MAX_INSTANCE_COUNT,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            label: Some("instance_buffer"),
        });

        let vs_module = device.create_shader_module(vk_shader_macros::include_glsl!(
            "src/shader.vert",
            kind: vert
        ));
        let fs_module = device.create_shader_module(vk_shader_macros::include_glsl!(
            "src/shader.frag",
            kind: frag
        ));

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &uniforms::Uniforms::layout(),
                label: Some("uniform_bind_group_layout"),
            });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            bindings: &uniforms.bindings(&uniform_buffer),
            label: Some("uniform_bind_group"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&uniform_bind_group_layout],
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                // TODO: Back
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            color_states: &[wgpu::ColorStateDescriptor {
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[
                    wgpu::VertexBufferDescriptor {
                        stride: (std::mem::size_of::<f32>() * 2) as wgpu::BufferAddress,
                        step_mode: wgpu::InputStepMode::Vertex,
                        attributes: &[wgpu::VertexAttributeDescriptor {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float2,
                        }],
                    },
                    wgpu::VertexBufferDescriptor {
                        stride: std::mem::size_of::<u32>() as wgpu::BufferAddress,
                        step_mode: wgpu::InputStepMode::Instance,
                        attributes: &[wgpu::VertexAttributeDescriptor {
                            offset: 0,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Uint,
                        }],
                    },
                ],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            uniforms,
            uniform_buffer,
            uniform_bind_group,
            size,
            camera,
            irdata: vec![].into(),
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
        self.camera.update_aspect(self.size.width, self.size.height);
    }

    // input() won't deal with GPU code, so it can be synchronous
    fn input(&mut self, _event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {
        self.uniforms.update_view_proj(&self.camera);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("update encoder"),
            });
        let staging_buffer = self.device.create_buffer_with_data(
            bytemuck::cast_slice(&[self.uniforms]),
            wgpu::BufferUsage::COPY_SRC,
        );
        encoder.copy_buffer_to_buffer(
            &staging_buffer,
            0,
            &self.uniform_buffer,
            0,
            std::mem::size_of::<uniforms::Uniforms>() as wgpu::BufferAddress,
        );
        self.queue.submit(&[encoder.finish()]);
    }

    fn render(&mut self) {
        let frame = self
            .swap_chain
            .get_next_texture()
            .expect("Timeout when acquiring next swap chain texture");
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Clear,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color::BLACK,
                }],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_vertex_buffer(0, &self.vertex_buffer, 0, 0);
            rpass.set_vertex_buffer(1, &self.instance_buffer, 0, 0);
            rpass.set_index_buffer(&self.index_buffer, 0, 0);
            rpass.set_bind_group(0, &self.uniform_bind_group, &[]);
            rpass.draw_indexed(0..36, 0, 0..self.uniforms.instance_count());
        }

        self.queue.submit(&[encoder.finish()]);
    }
}

async fn run(event_loop: EventLoop<IRData>, window: Window) {
    let mut gui = GUI::new(&window).await;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => window.request_redraw(),
            Event::RedrawRequested(_) => {
                gui.update();
                gui.render();
            }
            Event::UserEvent(IRData {
                buffer,
                width,
                height,
            }) => {
                assert_eq!(buffer.len(), width as usize * height as usize);
                gui.irdata = buffer;
                gui.uniforms.width = width;
                gui.uniforms.height = height;
                window.request_redraw();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !gui.input(event) {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            gui.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // new_inner_size is &mut so w have to dereference it twice
                            gui.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    });
}

fn main() {
    let event_loop = EventLoop::with_user_event();
    let window = winit::window::Window::new(&event_loop).unwrap();
    let proxy = event_loop.create_proxy();
    proxy
        .send_event(IRData {
            buffer: vec![8, 4, 100, 200, 47, 91].into(),
            width: 3,
            height: 2,
        })
        .unwrap();
    futures::executor::block_on(run(event_loop, window));
}

#[derive(Debug)]
pub struct IRData {
    pub buffer: Box<[u32]>,
    pub width: u32,
    pub height: u32,
}
