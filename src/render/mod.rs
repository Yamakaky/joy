use buffer::Staged;
use object::Vertex;
use std::sync::mpsc;
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

mod buffer;
mod camera;
mod object;
mod parameters;
mod texture;
mod uniforms;

fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_descs: &[wgpu::VertexBufferDescriptor],
    vs_spv: &[u32],
    fs_spv: &[u32],
    sample_count: u32,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: &layout,
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &device.create_shader_module(vs_spv),
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &device.create_shader_module(fs_spv),
            entry_point: "main",
        }),
        rasterization_state: Some(wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
        }),
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        color_states: &[wgpu::ColorStateDescriptor {
            format: color_format,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state: depth_format.map(|format| wgpu::DepthStencilStateDescriptor {
            format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
            stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
            stencil_read_mask: 0,
            stencil_write_mask: 0,
        }),
        sample_count,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: vertex_descs,
        },
    })
}

fn create_multisampled_framebuffer(
    device: &wgpu::Device,
    sc_desc: &wgpu::SwapChainDescriptor,
    sample_count: u32,
) -> wgpu::TextureView {
    let multisampled_texture_extent = wgpu::Extent3d {
        width: sc_desc.width,
        height: sc_desc.height,
        depth: 1,
    };
    let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
        size: multisampled_texture_extent,
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: sc_desc.format,
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        label: None,
    };

    device
        .create_texture(multisampled_frame_descriptor)
        .create_default_view()
}

struct GUI {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    render_pipeline: wgpu::RenderPipeline,
    sample_count: u32,
    multisampled_framebuffer: wgpu::TextureView,
    vertex_buffer: Staged<wgpu::Buffer>,
    uniforms: uniforms::Uniforms,
    uniform_buffer: Staged<wgpu::Buffer>,
    uniform_bind_group: wgpu::BindGroup,
    depth_texture: texture::Texture,
    size: winit::dpi::PhysicalSize<u32>,
    camera: camera::Camera,
    irdata: Box<[u8]>,
    ir_texture: Option<Staged<texture::Texture>>,
}

impl GUI {
    async fn new(window: &Window) -> Self {
        let sample_count = 16;
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
        let uniform_buffer = Staged::with_data(&device, &[uniforms], wgpu::BufferUsage::UNIFORM);

        let vertex_buffer = Staged::with_size(&device, Vertex::BUF_SIZE, wgpu::BufferUsage::VERTEX);

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

        let depth_texture = texture::Texture::create_depth_texture(
            &device,
            &sc_desc,
            sample_count,
            Some("depth_texture"),
        );

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&uniform_bind_group_layout],
        });

        let render_pipeline = create_render_pipeline(
            &device,
            &pipeline_layout,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            Some(texture::Texture::DEPTH_FORMAT),
            &[Vertex::descriptor()],
            vk_shader_macros::include_glsl!("src/render/shader.vert", kind: vert),
            vk_shader_macros::include_glsl!("src/render/shader.frag", kind: frag),
            sample_count,
        );
        let multisampled_framebuffer =
            create_multisampled_framebuffer(&device, &sc_desc, sample_count);

        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            render_pipeline,
            sample_count,
            multisampled_framebuffer,
            vertex_buffer,
            uniforms,
            uniform_buffer,
            uniform_bind_group,
            depth_texture,
            size,
            camera,
            irdata: vec![].into(),
            ir_texture: None,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
        self.multisampled_framebuffer =
            create_multisampled_framebuffer(&self.device, &self.sc_desc, self.sample_count);
        self.depth_texture = texture::Texture::create_depth_texture(
            &self.device,
            &self.sc_desc,
            self.sample_count,
            Some("depth_texture"),
        );
        self.camera.update_aspect(self.size.width, self.size.height);
    }

    // input() won't deal with GPU code, so it can be synchronous
    fn input(&mut self, event: &WindowEvent) -> bool {
        self.camera.input(event)
    }

    fn update(&mut self) {
        self.uniforms.update_view_proj(&self.camera);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("update encoder"),
            });
        self.uniform_buffer
            .update(&self.device, &mut encoder, &[self.uniforms]);
        self.queue.submit(&[encoder.finish()]);
    }

    fn push_ir_data(&mut self, ir_data: Box<[u8]>, width: u32, height: u32) {
        self.irdata = ir_data;
        self.uniforms.width = width;
        self.uniforms.height = height;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("IR data upload"),
            });
        let vertices = Vertex::from_ir(&self.irdata, self.uniforms.width, self.uniforms.height);
        self.vertex_buffer
            .update(&self.device, &mut encoder, &vertices);

        if self.ir_texture.is_none() {
            self.ir_texture = Some(Staged::new(texture::Texture::create_ir_texture(
                &self.device,
                (width, height),
            )));
        }
        self.ir_texture.as_mut().unwrap().update(
            &self.device,
            &mut encoder,
            &self.irdata,
            (width, height),
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
            let color_attachment = if self.sample_count == 1 {
                wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    load_op: wgpu::LoadOp::Clear,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color::BLACK,
                }
            } else {
                wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &self.multisampled_framebuffer,
                    resolve_target: Some(&frame.view),
                    load_op: wgpu::LoadOp::Clear,
                    store_op: wgpu::StoreOp::Store,
                    clear_color: wgpu::Color::BLACK,
                }
            };
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[color_attachment],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &self.depth_texture.view,
                    depth_load_op: wgpu::LoadOp::Clear,
                    depth_store_op: wgpu::StoreOp::Store,
                    clear_depth: 1.0,
                    stencil_load_op: wgpu::LoadOp::Clear,
                    stencil_store_op: wgpu::StoreOp::Store,
                    clear_stencil: 0,
                }),
            });
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_vertex_buffer(0, &self.vertex_buffer, 0, 0);
            rpass.set_bind_group(0, &self.uniform_bind_group, &[]);
            if self.uniforms.width > 0 && self.uniforms.height > 0 {
                rpass.draw(
                    0..(self.uniforms.width - 1) * (self.uniforms.height - 1) * 6,
                    0..1,
                );
            } else {
                rpass.draw(0..0, 0..1);
            }
        }

        self.queue.submit(&[encoder.finish()]);
    }
}

pub async fn run(
    event_loop: EventLoop<IRData>,
    window: Window,
    thread_contact: mpsc::Sender<JoyconCmd>,
    thread_handle: std::thread::JoinHandle<anyhow::Result<()>>,
) -> ! {
    let mut gui = GUI::new(&window).await;
    window.set_maximized(true);

    let mut thread_handle = Some(thread_handle);

    let mut parameters = parameters::Parameters::new();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::MainEventsCleared => window.request_redraw(),
            Event::LoopDestroyed => {
                eprintln!("sending shutdown signal to thread");
                let _ = thread_contact.send(JoyconCmd::Stop);
                match thread_handle
                    .take()
                    .expect("thread already exited???")
                    .join()
                {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => eprintln!("Joycon thread exited with error: {:?}", e),
                    Err(_) => eprintln!("Joycon thread crashed"),
                }
            }
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
                gui.push_ir_data(buffer, width, height);
                window.request_redraw();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if gui.input(event) || parameters.input(event, &thread_contact) {
                    window.request_redraw();
                } else {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
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

#[derive(Debug)]
pub struct IRData {
    pub buffer: Box<[u8]>,
    pub width: u32,
    pub height: u32,
}

pub enum JoyconCmd {
    Stop,
    SetResolution(joycon_sys::mcu::ir::Resolution),
    SetRegister(joycon_sys::mcu::ir::Register),
}
