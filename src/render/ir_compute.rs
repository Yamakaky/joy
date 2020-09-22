use iced_wgpu::wgpu;

pub struct IRCompute {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    pub texture_binding_layout: wgpu::BindGroupLayout,
    texture: Option<super::texture::Texture>,
    pub normal_texture: super::texture::Texture,
    normal_texture_binding_layout: wgpu::BindGroupLayout,
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
        // TODO: remove magic value
        let vertex_buffer_size = 320 * 240 * 16 * 3;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("compute vertex buffer"),
            size: vertex_buffer_size,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::STORAGE,
            mapped_at_creation: false,
        });
        let index_buffer_size = 320 * 240 * 6 * 4;
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("compute index buffer"),
            size: index_buffer_size,
            usage: wgpu::BufferUsage::INDEX | wgpu::BufferUsage::STORAGE,
            mapped_at_creation: false,
        });
        let mesh_binding_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            readonly: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            readonly: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("static compute binding layout"),
            });
        let mesh_binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &mesh_binding_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(vertex_buffer.slice(..)),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(index_buffer.slice(..)),
                },
            ],
            label: Some("static compute binding"),
        });
        let texture_binding_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE | wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            dimension: wgpu::TextureViewDimension::D2,
                            component_type: wgpu::TextureComponentType::Uint,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::COMPUTE | wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                        count: None,
                    },
                ],
                label: Some("dynamic compute binding layout"),
            });
        let normal_texture_binding_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::COMPUTE | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::StorageTexture {
                        dimension: wgpu::TextureViewDimension::D2,
                        readonly: false,
                        format: wgpu::TextureFormat::Rgba32Float,
                    },
                    count: None,
                }],
                label: Some("Depth texture binding layout"),
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[
                &mesh_binding_layout,
                &texture_binding_layout,
                &normal_texture_binding_layout,
                uniform_bind_group_layout,
            ],
            push_constant_ranges: &[],
            label: Some("IR Compute Pipeline Layout"),
        });
        let spirv: &[u32] = vk_shader_macros::include_glsl!("src/render/shaders/compute.comp");
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            layout: Some(&pipeline_layout),
            compute_stage: wgpu::ProgrammableStageDescriptor {
                module: &device.create_shader_module(wgpu::ShaderModuleSource::SpirV(spirv.into())),
                entry_point: "main",
            },
            label: Some("IR Compute Pipeline"),
        });
        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            mesh_binding,
            texture_binding_layout,
            texture: None,
            texture_binding: None,
            normal_texture: super::texture::Texture::create_normal_texture(device, (1, 1)),
            normal_texture_binding_layout,
        }
    }

    pub fn push_ir_data(
        &mut self,
        device: &wgpu::Device,
        queue: &mut wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        uniform_bind_group: &wgpu::BindGroup,
        image: image::GrayImage,
    ) {
        let (width, height) = image.dimensions();
        let texture = self.texture.get_or_insert_with(|| {
            super::texture::Texture::create_ir_texture(device, (width, height))
        });
        texture.update(device, queue, image);

        if self.normal_texture.size.width != width || self.normal_texture.size.height != height {
            self.normal_texture =
                super::texture::Texture::create_normal_texture(device, (width, height));
        }

        let texture_binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.texture_binding_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
            label: Some("dynamic compute group"),
        });
        let normal_texture_binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.normal_texture_binding_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&self.normal_texture.view),
            }],
            label: Some("Normal texture bind group"),
        });

        {
            let mut cpass = encoder.begin_compute_pass();
            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, &self.mesh_binding, &[]);
            cpass.set_bind_group(1, &texture_binding, &[]);
            cpass.set_bind_group(2, &normal_texture_binding, &[]);
            cpass.set_bind_group(3, uniform_bind_group, &[]);
            cpass.dispatch(width, height, 1);
        }

        self.texture_binding = Some(texture_binding);
    }
}
