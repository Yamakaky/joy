use iced_wgpu::{wgpu, Backend, Renderer, Settings, Viewport};
use iced_winit::{
    futures, program,
    winit::{
        dpi::{PhysicalPosition, PhysicalSize},
        event::{
            DeviceEvent, ElementState, Event, KeyboardInput, ModifiersState, StartCause,
            VirtualKeyCode, WindowEvent,
        },
        event_loop::{ControlFlow, EventLoop},
        window::Window,
    },
    Debug, Size,
};
use joycon::joycon_sys;
use std::sync::mpsc;
use std::time::{Duration, Instant};

mod buffer;
mod camera;
mod controls;
mod d2;
mod d3;
mod ir_compute;
mod texture;
mod uniforms;

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
            cull_mode: wgpu::CullMode::Back,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
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
            stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
            stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
            stencil_read_mask: 0,
            stencil_write_mask: 0,
        }),
        sample_count,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint32,
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
        mip_level_count: 1,
        array_layer_count: 1,
        sample_count,
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
    viewport: Viewport,
    swap_chain: wgpu::SwapChain,
    sample_count: u32,
    multisampled_framebuffer: wgpu::TextureView,
    pointer_target: wgpu::Texture,
    pointer_target_view: wgpu::TextureView,
    mouse_position: PhysicalPosition<f64>,
    uniforms: uniforms::UniformHandler,
    camera: camera::Camera,
    compute: ir_compute::IRCompute,
    render_d2: d2::D2,
    render_d3: d3::D3,
    iced_renderer: Renderer,
    interface: program::State<controls::Controls>,
    iced_debug: Debug,
    staging_depth_buffer: wgpu::Buffer,
}

impl GUI {
    fn new(window: &Window, thread_contact: mpsc::Sender<JoyconCmd>) -> Self {
        let sample_count = 2;
        let size = window.inner_size();
        let viewport =
            Viewport::with_physical_size(Size::new(size.width, size.height), window.scale_factor());
        let surface = wgpu::Surface::create(window);

        let adapter = futures::executor::block_on(wgpu::Adapter::request(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            },
            wgpu::BackendBit::VULKAN,
        ))
        .unwrap();

        let (device, queue) =
            futures::executor::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                extensions: wgpu::Extensions {
                    anisotropic_filtering: false,
                },
                limits: wgpu::Limits::default(),
            }));

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let camera = camera::Camera::new(&sc_desc);
        let mut uniforms = uniforms::UniformHandler::new(&device);
        uniforms.update_view_proj(&camera);

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
            array_layer_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::COPY_SRC,
            label: None,
        };
        let pointer_target = device.create_texture(multisampled_frame_descriptor);
        let pointer_target_view = pointer_target.create_default_view();

        // Initialize iced
        let mut iced_debug = Debug::new();
        let mut iced_renderer = Renderer::new(Backend::new(&device, Settings::default()));

        let compute = ir_compute::IRCompute::new(&device, uniforms.bind_group_layout());
        let render_d2 = d2::D2::new(&device, &compute.texture_binding_layout, sample_count);
        let render_d3 = d3::D3::new(&device, &uniforms, &compute, &sc_desc, sample_count);
        let interface = program::State::new(
            controls::Controls::new(thread_contact),
            viewport.logical_size(),
            &mut iced_renderer,
            &mut iced_debug,
        );

        let staging_depth_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("depth reader"),
            size: 1,
            usage: wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::MAP_READ,
        });

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
            camera,
            compute,
            render_d2,
            render_d3,
            iced_renderer,
            interface,
            iced_debug,
            staging_depth_buffer,
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
            array_layer_count: 1,
            sample_count: self.sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::COPY_SRC,
            label: None,
        };

        self.pointer_target = self.device.create_texture(multisampled_frame_descriptor);
        self.pointer_target_view = self.pointer_target.create_default_view();
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
            None,
            self.viewport.logical_size(),
            &mut self.iced_renderer,
            &mut self.iced_debug,
        );
        self.render_d3.update_bindgroup(&self.device, &self.compute);
    }

    fn copy_depth(&mut self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_texture_to_buffer(
            wgpu::TextureCopyView {
                texture: &self.pointer_target,
                mip_level: 0,
                array_layer: 0,
                origin: wgpu::Origin3d {
                    x: self.mouse_position.x as u32,
                    y: self.mouse_position.y as u32,
                    z: 0,
                },
            },
            wgpu::BufferCopyView {
                buffer: &self.staging_depth_buffer,
                offset: 0,
                bytes_per_row: 1,
                rows_per_image: 1,
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth: 1,
            },
        );
    }

    async fn get_depth(&mut self) {
        let mapping_future = self.staging_depth_buffer.map_read(0, 1);
        self.device.poll(wgpu::Maintain::Wait);
        let mapping = mapping_future.await.unwrap();
        self.interface.queue_message(controls::Message::Depth(
            self.mouse_position.x as u32,
            self.mouse_position.y as u32,
            mapping.as_slice()[0],
        ));
    }

    fn push_ir_data(&mut self, image: image::GrayImage) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("IR compute shader"),
            });
        self.uniforms.upload(&self.device, &mut encoder);
        self.compute.push_ir_data(
            &self.device,
            &mut encoder,
            &self.uniforms.bind_group(),
            image,
        );
        self.queue.submit(&[encoder.finish()]);
    }

    fn render(&mut self, window: &Window) {
        let frame = self
            .swap_chain
            .get_next_texture()
            .expect("Timeout when acquiring next swap chain texture");

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Rendering"),
            });

        self.uniforms.upload(&self.device, &mut encoder);

        {
            let mut rpass3d = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[
                    self.color_attachment(&frame, wgpu::LoadOp::Clear),
                    wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &self.pointer_target_view,
                        resolve_target: None,
                        load_op: wgpu::LoadOp::Clear,
                        store_op: wgpu::StoreOp::Store,
                        clear_color: wgpu::Color::RED,
                    },
                ],
                depth_stencil_attachment: Some(self.render_d3.depth_stencil_attachement()),
            });
            if self.compute.texture_binding.is_some() {
                self.render_d3
                    .render(&mut rpass3d, &self.compute, &self.uniforms);
            }
        }

        self.copy_depth(&mut encoder);

        if let Some(ref texture) = self.compute.texture_binding {
            let mut rpass2d = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[self.color_attachment(&frame, wgpu::LoadOp::Load)],
                depth_stencil_attachment: None,
            });
            self.render_d2.render(&mut rpass2d, texture);
        }

        let mouse_interaction = self.iced_renderer.backend_mut().draw(
            &self.device,
            &mut encoder,
            &frame.view,
            &self.viewport,
            self.interface.primitive(),
            &self.iced_debug.overlay(),
        );

        // [2020-06-09T18:01:01Z ERROR gfx_backend_vulkan] [Validation] Validation Error: [ VUID-vkQueuePresentKHR-pWaitSemaphores-03268 ] Object 0: handle = 0x1bd68ec4ec8, type = VK_OBJECT_TYPE_QUEUE; Object 1: handle = 0xd76249000000000c, type = VK_OBJECT_TYPE_SEMAPHORE; | MessageID = 0x251f8f7a | VkQueue 0x1bd68ec4ec8[] is waiting on VkSemaphore 0xd76249000000000c[] that has no way to be signaled. The Vulkan spec states: All elements of the pWaitSemaphores member of pPresentInfo must reference a semaphore signal operation that has been submitted for execution and any semaphore signal operations on which it depends (if any) must have also been submitted for execution. (https://www.khronos.org/registry/vulkan/specs/1.1-extensions/html/vkspec.html#VUID-vkQueuePresentKHR-pWaitSemaphores-03268)
        futures::executor::block_on(self.get_depth());
        self.queue.submit(&[encoder.finish()]);

        // And update the mouse cursor
        window.set_cursor_icon(iced_winit::conversion::mouse_interaction(mouse_interaction));
    }

    fn color_attachment<'a>(
        &'a self,
        frame: &'a wgpu::SwapChainOutput,
        load_op: wgpu::LoadOp,
    ) -> wgpu::RenderPassColorAttachmentDescriptor {
        if self.sample_count == 1 {
            wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.view,
                resolve_target: None,
                load_op,
                store_op: wgpu::StoreOp::Store,
                clear_color: wgpu::Color::BLUE,
            }
        } else {
            wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &self.multisampled_framebuffer,
                resolve_target: Some(&frame.view),
                load_op,
                store_op: wgpu::StoreOp::Store,
                clear_color: wgpu::Color::BLUE,
            }
        }
    }
}

pub fn run(
    event_loop: EventLoop<JoyconData>,
    window: Window,
    thread_contact: mpsc::Sender<JoyconCmd>,
    _thread_handle: std::thread::JoinHandle<anyhow::Result<()>>,
) -> ! {
    let mut gui = GUI::new(&window, thread_contact.clone());

    let mut hidden = false;

    fn set_grabbed(window: &Window, grabbed: bool) -> bool {
        window.set_cursor_grab(grabbed).unwrap();
        window.set_cursor_visible(!grabbed);
        grabbed
    };
    let mut grabbed = set_grabbed(&window, false);

    let mut last_tick = Instant::now();
    let mut modifiers = ModifiersState::default();

    let mut frame_count = 0;
    let mut frame_counter = Instant::now();

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
                frame_count += 1;
                if frame_counter.elapsed() > Duration::from_secs(1) {
                    println!("{} fps", frame_count);
                    frame_counter = Instant::now();
                    frame_count = 0;
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
                    gui.render(&window);
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
            Event::UserEvent(JoyconData::IRImage(image, position)) => {
                gui.push_ir_data(image);
                if gui.interface.program().ir_rotate() {
                    gui.uniforms.set_ir_rotation(position.rotation);
                } else {
                    use cgmath::prelude::One;
                    gui.uniforms.set_ir_rotation(cgmath::Quaternion::one());
                }
                window.request_redraw();
            }
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
                            if physical_size.height != 0 && physical_size.height != 0 {
                                hidden = false;
                                gui.resize(&window, *physical_size);
                            } else {
                                hidden = true;
                            }
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            if new_inner_size.height != 0 && new_inner_size.height != 0 {
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

pub enum JoyconData {
    IRImage(image::GrayImage, joycon::Position),
}

pub enum JoyconCmd {
    Stop,
    SetResolution(joycon_sys::mcu::ir::Resolution),
    SetRegister(joycon_sys::mcu::ir::Register),
    SetRegisters([joycon_sys::mcu::ir::Register; 2]),
}
