use buffer::BoundBuffer;
use iced_core::Point;
use iced_wgpu::{wgpu, Backend, Renderer, Settings, Viewport};
use iced_winit::{
    program,
    winit::{
        dpi::{PhysicalPosition, PhysicalSize},
        event::{
            DeviceEvent, ElementState, Event, KeyboardInput, ModifiersState, StartCause,
            VirtualKeyCode, WindowEvent,
        },
        event_loop::{ControlFlow, EventLoop, EventLoopProxy},
        window::Window,
    },
    Debug, Size,
};
use joycon::joycon_sys;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use uniforms::{Lights, Uniforms};

mod buffer;
mod camera;
mod controls;
mod d2;
mod d3;
mod ir_compute;
mod texture;
mod uniforms;

#[allow(clippy::too_many_arguments)]
pub fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_formats: &[wgpu::TextureFormat],
    depth_format: Option<wgpu::TextureFormat>,
    vertex_descs: &[wgpu::VertexBufferDescriptor],
    vs_spv: &[u32],
    fs_spv: &[u32],
    sample_count: u32,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: Some(layout),
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &device.create_shader_module(wgpu::ShaderModuleSource::SpirV(vs_spv.into())),
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &device.create_shader_module(wgpu::ShaderModuleSource::SpirV(fs_spv.into())),
            entry_point: "main",
        }),
        rasterization_state: Some(wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::Back,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
            clamp_depth: false,
        }),
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        color_states: &color_formats
            .iter()
            .map(|color_format| wgpu::ColorStateDescriptor {
                format: *color_format,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            })
            .collect::<Vec<_>>(),
        depth_stencil_state: depth_format.map(|format| wgpu::DepthStencilStateDescriptor {
            format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilStateDescriptor {
                front: wgpu::StencilStateFaceDescriptor::IGNORE,
                back: wgpu::StencilStateFaceDescriptor::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
        }),
        sample_count,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint32,
            vertex_buffers: vertex_descs,
        },
        label: Some("Render Pipeline"),
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
        label: Some("Output Framebuffer Descriptor"),
        size: multisampled_texture_extent,
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: sc_desc.format,
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
    };

    device
        .create_texture(multisampled_frame_descriptor)
        .create_view(&wgpu::TextureViewDescriptor {
            label: Some("Output Framebuffer View"),
            dimension: Some(wgpu::TextureViewDimension::D2),
            format: None,
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        })
}

struct GUI {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    viewport: Viewport,
    swap_chain: wgpu::SwapChain,
    sample_count: u32,
    multisampled_framebuffer: wgpu::TextureView,
    pointer_target: wgpu::Texture,
    pointer_target_view: wgpu::TextureView,
    mouse_position: PhysicalPosition<f64>,
    uniforms: BoundBuffer<Uniforms>,
    lights: BoundBuffer<Lights>,
    camera: camera::Camera,
    compute: ir_compute::IRCompute,
    render_d2: d2::D2,
    render_d3: d3::D3,
    iced_renderer: Renderer,
    interface: program::State<controls::Controls>,
    iced_debug: Debug,
    staging_depth_buffer_send: async_channel::Sender<wgpu::Buffer>,
    staging_depth_buffer_recv: async_channel::Receiver<wgpu::Buffer>,
    staging_belt: wgpu::util::StagingBelt,
}

impl GUI {
    async fn new(window: &Window, thread_contact: mpsc::Sender<JoyconCmd>) -> Self {
        let sample_count = 1;
        let size = window.inner_size();
        let viewport =
            Viewport::with_physical_size(Size::new(size.width, size.height), window.scale_factor());

        let instance = wgpu::Instance::new(wgpu::BackendBit::VULKAN);

        let surface = unsafe { instance.create_surface(window) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    limits: wgpu::Limits::default(),
                    features: wgpu::Features::empty(),
                    shader_validation: true,
                },
                None,
            )
            .await
            .unwrap();

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let camera = camera::Camera::new(&sc_desc);
        let mut uniforms = BoundBuffer::<Uniforms>::new(
            &device,
            wgpu::BufferUsage::UNIFORM,
            wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT | wgpu::ShaderStage::COMPUTE,
        );
        uniforms.update_view_proj(&camera);

        let mut lights = BoundBuffer::<Lights>::new(
            &device,
            wgpu::BufferUsage::UNIFORM,
            wgpu::ShaderStage::FRAGMENT,
        );
        *lights = Lights::lights();

        let multisampled_framebuffer =
            create_multisampled_framebuffer(&device, &sc_desc, sample_count);

        let multisampled_texture_extent = wgpu::Extent3d {
            width: sc_desc.width,
            height: sc_desc.height,
            depth: 1,
        };
        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            size: multisampled_texture_extent,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::COPY_SRC,
            label: Some("Pointer Texture Descriptor"),
        };
        let pointer_target = device.create_texture(multisampled_frame_descriptor);
        let pointer_target_view = pointer_target.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Pointer Texture"),
            dimension: Some(wgpu::TextureViewDimension::D2),
            format: Some(wgpu::TextureFormat::R8Unorm),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        // Initialize iced
        let mut iced_debug = Debug::new();
        let mut iced_renderer = Renderer::new(Backend::new(&device, Settings::default()));

        let compute = ir_compute::IRCompute::new(&device, uniforms.bind_group_layout());
        let render_d2 = d2::D2::new(&device, &compute.texture_binding_layout, sample_count);
        let render_d3 = d3::D3::new(
            &device,
            &uniforms,
            &lights,
            &compute,
            &sc_desc,
            sample_count,
        );
        let interface = program::State::new(
            controls::Controls::new(thread_contact),
            viewport.logical_size(),
            // TODO
            Point::ORIGIN,
            &mut iced_renderer,
            &mut iced_debug,
        );

        let (staging_depth_buffer_send, staging_depth_buffer_recv) = async_channel::unbounded();
        // TODO: multiple multi-use staging buffers
        for _ in 0..5 {
            staging_depth_buffer_send
                .send(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("depth reader"),
                    size: wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as wgpu::BufferAddress,
                    usage: wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::MAP_READ,
                    mapped_at_creation: false,
                }))
                .await
                .unwrap();
        }

        Self {
            surface,
            device,
            queue,
            sc_desc,
            viewport,
            swap_chain,
            sample_count,
            multisampled_framebuffer,
            pointer_target,
            pointer_target_view,
            mouse_position: PhysicalPosition::new(0., 0.),
            uniforms,
            lights,
            camera,
            compute,
            render_d2,
            render_d3,
            iced_renderer,
            interface,
            iced_debug,
            staging_depth_buffer_send,
            staging_depth_buffer_recv,
            staging_belt: wgpu::util::StagingBelt::new(1024 * 1024),
        }
    }

    fn resize(&mut self, window: &Window, new_size: PhysicalSize<u32>) {
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
        self.multisampled_framebuffer =
            create_multisampled_framebuffer(&self.device, &self.sc_desc, self.sample_count);
        self.render_d3
            .resize(&self.device, &self.sc_desc, self.sample_count);
        self.camera
            .update_aspect(self.sc_desc.width, self.sc_desc.height);

        self.viewport = Viewport::with_physical_size(
            Size::new(new_size.width, new_size.height),
            window.scale_factor(),
        );

        let multisampled_texture_extent = wgpu::Extent3d {
            width: self.sc_desc.width,
            height: self.sc_desc.height,
            depth: 1,
        };
        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            size: multisampled_texture_extent,
            mip_level_count: 1,
            sample_count: self.sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::COPY_SRC,
            label: Some("Pointer Texture Descriptor"),
        };

        self.pointer_target = self.device.create_texture(multisampled_frame_descriptor);
        self.pointer_target_view = self
            .pointer_target
            .create_view(&wgpu::TextureViewDescriptor {
                label: Some("Pointer Texture"),
                dimension: Some(wgpu::TextureViewDimension::D2),
                format: Some(wgpu::TextureFormat::R8Unorm),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            });
        self.mouse_position = PhysicalPosition::new(0., 0.);
    }

    // input() won't deal with GPU code, so it can be synchronous
    fn input(&mut self, event: &WindowEvent) -> bool {
        self.camera.input(event)
    }

    fn update(&mut self, dt: Duration) {
        self.camera.update(dt);
        self.uniforms.update_view_proj(&self.camera);
        let _ = self.interface.update(
            self.viewport.logical_size(),
            Point::new(self.mouse_position.x as f32, self.mouse_position.y as f32),
            None,
            &mut self.iced_renderer,
            &mut self.iced_debug,
        );
        self.render_d3.update_bindgroup(&self.device, &self.compute);
    }

    fn copy_depth(&mut self, encoder: &mut wgpu::CommandEncoder, buffer: &wgpu::Buffer) {
        encoder.copy_texture_to_buffer(
            wgpu::TextureCopyView {
                texture: &self.pointer_target,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: self.mouse_position.x as u32,
                    y: self.mouse_position.y as u32,
                    z: 0,
                },
            },
            wgpu::BufferCopyView {
                buffer,
                layout: wgpu::TextureDataLayout {
                    offset: 0,
                    bytes_per_row: wgpu::COPY_BYTES_PER_ROW_ALIGNMENT,
                    rows_per_image: 1,
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth: 1,
            },
        );
    }

    fn push_ir_data(&mut self, image: image::GrayImage) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("IR compute shader"),
            });
        self.uniforms.upload(&mut self.queue);
        self.compute.push_ir_data(
            &self.device,
            &mut self.queue,
            &mut encoder,
            &self.uniforms.bind_group(),
            image,
        );
        self.queue.submit(Some(encoder.finish()));
    }

    fn render(&mut self, window: &Window, proxy: EventLoopProxy<UserEvent>) {
        let frame = self
            .swap_chain
            .get_current_frame()
            .expect("Timeout when acquiring next swap chain texture");

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Rendering"),
            });

        self.uniforms.upload(&mut self.queue);
        self.lights.upload(&mut self.queue);

        {
            let mut rpass3d = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[
                    self.color_attachment(&frame, wgpu::LoadOp::Clear(wgpu::Color::BLUE)),
                    wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &self.pointer_target_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::RED),
                            store: true,
                        },
                    },
                ],
                depth_stencil_attachment: Some(self.render_d3.depth_stencil_attachement()),
            });
            if self.compute.texture_binding.is_some() {
                self.render_d3
                    .render(&mut rpass3d, &self.compute, &self.uniforms, &self.lights);
            }
        }

        let staging_depth_buffer = self.staging_depth_buffer_recv.try_recv();
        if let Ok(ref buffer) = staging_depth_buffer {
            self.copy_depth(&mut encoder, buffer);
        }

        if let Some(ref texture) = self.compute.texture_binding {
            let mut rpass2d = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[self.color_attachment(&frame, wgpu::LoadOp::Load)],
                depth_stencil_attachment: None,
            });
            self.render_d2.render(&mut rpass2d, texture);
        }

        let mouse_interaction = self.iced_renderer.backend_mut().draw(
            &self.device,
            &mut self.staging_belt,
            &mut encoder,
            &frame.output.view,
            &self.viewport,
            self.interface.primitive(),
            &self.iced_debug.overlay(),
        );

        self.staging_belt.finish();
        self.queue.submit(Some(encoder.finish()));
        smol::spawn(self.staging_belt.recall()).detach();

        if let Ok(buffer) = staging_depth_buffer {
            // Update the depth picker at cursor position
            let (x, y) = (self.mouse_position.x as u32, self.mouse_position.y as u32);
            let sender = self.staging_depth_buffer_send.clone();

            smol::spawn(async move {
                {
                    let slice = buffer.slice(0..wgpu::COPY_BUFFER_ALIGNMENT);
                    slice.map_async(wgpu::MapMode::Read).await.unwrap();
                    let _ = proxy.send_event(UserEvent::Message(controls::Message::Depth(
                        x,
                        y,
                        slice.get_mapped_range()[0],
                    )));
                    buffer.unmap();
                }
                sender.send(buffer).await.unwrap();
            })
            .detach();
        }

        // And update the mouse cursor
        window.set_cursor_icon(iced_winit::conversion::mouse_interaction(mouse_interaction));
    }

    fn color_attachment<'a>(
        &'a self,
        frame: &'a wgpu::SwapChainFrame,
        load_op: wgpu::LoadOp<wgpu::Color>,
    ) -> wgpu::RenderPassColorAttachmentDescriptor {
        if self.sample_count == 1 {
            wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.output.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: load_op,
                    store: true,
                },
            }
        } else {
            wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &self.multisampled_framebuffer,
                resolve_target: Some(&frame.output.view),
                ops: wgpu::Operations {
                    load: load_op,
                    store: true,
                },
            }
        }
    }
}

pub async fn run(
    event_loop: EventLoop<UserEvent>,
    window: Window,
    thread_contact: mpsc::Sender<JoyconCmd>,
    _thread_handle: std::thread::JoinHandle<()>,
) -> ! {
    let mut gui = GUI::new(&window, thread_contact.clone()).await;

    let mut hidden = false;

    fn set_grabbed(window: &Window, grabbed: bool) -> bool {
        window.set_cursor_grab(grabbed).unwrap();
        window.set_cursor_visible(!grabbed);
        grabbed
    };
    let mut grabbed = set_grabbed(&window, false);

    let mut last_tick = Instant::now();
    let mut modifiers = ModifiersState::default();

    let mut _frame_count = 0;
    let mut frame_counter = Instant::now();

    let proxy = event_loop.create_proxy();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::NewEvents(StartCause::Init) => {
                last_tick = Instant::now();
            }
            Event::Suspended => {
                dbg!("suspended");
            }
            Event::Resumed => {
                dbg!("resumed");
            }
            Event::RedrawEventsCleared => {
                _frame_count += 1;
                if frame_counter.elapsed() > Duration::from_secs(1) {
                    //println!("{} fps", frame_count);
                    frame_counter = Instant::now();
                    _frame_count = 0;
                }
            }
            Event::MainEventsCleared => {
                if !hidden {
                    gui.update(last_tick.elapsed());
                }
                last_tick = Instant::now();
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                if !hidden {
                    gui.render(&window, proxy.clone());
                }
            }
            Event::LoopDestroyed => {
                eprintln!("sending shutdown signal to thread");
                let _ = thread_contact.send(JoyconCmd::Stop);
                // TODO: join thread with timeout
                std::thread::sleep(Duration::from_millis(500));
                /*match thread_handle
                    .take()
                    .expect("thread already exited???")
                    .join()
                {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => eprintln!("Joycon thread exited with error: {:?}", e),
                    Err(_) => eprintln!("Joycon thread crashed"),
                }*/
            }
            Event::UserEvent(e) => match e {
                UserEvent::IRImage(image, position) => {
                    gui.push_ir_data(image);
                    if gui.interface.program().ir_rotate() {
                        gui.uniforms.set_ir_rotation(position.rotation);
                    } else {
                        use cgmath::prelude::One;
                        gui.uniforms.set_ir_rotation(cgmath::Quaternion::one());
                    }
                    window.request_redraw();
                }
                UserEvent::Message(m) => {
                    gui.interface.queue_message(m);
                }
            },
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } if grabbed => {
                gui.camera.mouse_move(delta);
                window.request_redraw();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if gui.input(event) {
                    window.request_redraw();
                } else {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::ModifiersChanged(new_modifiers) => {
                            modifiers = *new_modifiers;
                        }
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Space),
                                    ..
                                },
                            ..
                        } => {
                            grabbed = set_grabbed(&window, !grabbed);
                        }
                        WindowEvent::Resized(physical_size) => {
                            if physical_size.height != 0 && physical_size.width != 0 {
                                hidden = false;
                                gui.resize(&window, *physical_size);
                            } else {
                                hidden = true;
                            }
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            if new_inner_size.height != 0 && new_inner_size.width != 0 {
                                hidden = false;
                                gui.resize(&window, **new_inner_size);
                            } else {
                                hidden = true;
                            }
                        }
                        WindowEvent::Focused(false) => {
                            grabbed = set_grabbed(&window, false);
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            gui.mouse_position = *position;
                        }
                        _ => {}
                    }
                }

                // Map window event to iced event
                if let Some(event) =
                    iced_winit::conversion::window_event(&event, window.scale_factor(), modifiers)
                {
                    gui.interface.queue_event(event);
                }
            }
            _ => {}
        }
    });
}

pub enum UserEvent {
    IRImage(image::GrayImage, joycon::Position),
    Message(controls::Message),
}

pub enum JoyconCmd {
    Stop,
    SetResolution(joycon_sys::mcu::ir::Resolution),
    SetRegister(joycon_sys::mcu::ir::Register),
    SetRegisters([joycon_sys::mcu::ir::Register; 2]),
}
