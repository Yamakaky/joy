use iced_wgpu::wgpu;

pub struct IRCompute {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    pub texture_binding_layout: wgpu::BindGroupLayout,
    texture: Option<super::texture::Texture>,
    pub texture_binding: Option<wgpu::BindGroup>,
    mesh_binding: wgpu::BindGroup,
    pipeline: wgpu::ComputePipeline,
}

impl IRCompute {
    pub fn vertices(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub fn indices(&self) -> &wgpu::Buffer {
        &self.index_buffer
    }

    pub fn indices_count(&self) -> u32 {
        self.texture
            .as_ref()
            .map(|texture| (texture.size.width - 1) * (texture.size.height - 1) * 6)
            .unwrap_or(0)
    }

    pub fn new(device: &wgpu::Device, uniform_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        let vertex_buffer_size = 320 * 240 * 2 * 4 * 4;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("compute vertex buffer"),
            size: vertex_buffer_size,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::STORAGE,
        });
        let index_buffer_size = 320 * 240 * 6 * 4;
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("compute index buffer"),
            size: index_buffer_size,
            usage: wgpu::BufferUsage::INDEX | wgpu::BufferUsage::STORAGE,
        });
        let mesh_binding_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            readonly: false,
                        },
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            readonly: false,
                        },
                    },
                ],
                label: Some("static compute binding layout"),
            });
        let mesh_binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &mesh_binding_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &vertex_buffer,
                        range: 0..vertex_buffer_size,
                    },
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &index_buffer,
                        range: 0..index_buffer_size,
                    },
                },
            ],
            label: Some("static compute binding"),
        });
        let texture_binding_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE | wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            dimension: wgpu::TextureViewDimension::D2,
                            component_type: wgpu::TextureComponentType::Uint,
                            multisampled: false,
                        },
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::COMPUTE | wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                    },
                ],
                label: Some("dynamic compute binding layout"),
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[
                &mesh_binding_layout,
                &texture_binding_layout,
                uniform_bind_group_layout,
            ],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            layout: &pipeline_layout,
            compute_stage: wgpu::ProgrammableStageDescriptor {
                module: &device.create_shader_module(vk_shader_macros::include_glsl!(
                    "src/render/shaders/compute.comp",
                    kind: comp
                )),
                entry_point: "main",
            },
        });
        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            mesh_binding,
            texture_binding_layout,
            texture: None,
            texture_binding: None,
        }
    }

    pub fn push_ir_data(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        uniform_bind_group: &wgpu::BindGroup,
        image: image::GrayImage,
    ) {
        let (width, height) = image.dimensions();
        let texture = self.texture.get_or_insert_with(|| {
            super::texture::Texture::create_ir_texture(device, (width, height))
        });
        texture.update(device, encoder, image);

        let texture_binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.texture_binding_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
            label: Some("dynamic compute group"),
        });

        {
            let mut cpass = encoder.begin_compute_pass();
            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, &self.mesh_binding, &[]);
            cpass.set_bind_group(1, &texture_binding, &[]);
            cpass.set_bind_group(2, uniform_bind_group, &[]);
            cpass.dispatch(width, height, 1);
        }

        self.texture_binding = Some(texture_binding);
    }
}
