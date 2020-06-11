use super::{buffer::BoundBuffer, ir_compute::IRCompute, texture::Texture, uniforms::Uniforms};
use iced_wgpu::wgpu;

pub struct D3 {
    pipeline: wgpu::RenderPipeline,
    depth_texture: Texture,
    normal_texture_binding: wgpu::BindGroup,
    normal_texture_binding_layout: wgpu::BindGroupLayout,
}

impl D3 {
    pub fn new(
        device: &wgpu::Device,
        uniforms: &BoundBuffer<Uniforms>,
        compute: &IRCompute,
        sc_desc: &wgpu::SwapChainDescriptor,
        sample_count: u32,
    ) -> Self {
        let depth_texture =
            Texture::create_depth_texture(&device, &sc_desc, sample_count, Some("Depth texture"));

        let normal_texture_binding_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            dimension: wgpu::TextureViewDimension::D2,
                            component_type: wgpu::TextureComponentType::Float,
                            multisampled: false,
                        },
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                    },
                ],
                label: Some("Normal texture 3D pipeline"),
            });
        let normal_texture_binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &normal_texture_binding_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&compute.normal_texture.view),
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&compute.normal_texture.sampler),
                },
            ],
            label: Some("Normal texture sampled"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[uniforms.bind_group_layout(), &normal_texture_binding_layout],
        });

        let pipeline = super::create_render_pipeline(
            &device,
            &pipeline_layout,
            &[
                wgpu::TextureFormat::Bgra8UnormSrgb,
                wgpu::TextureFormat::R8Unorm,
            ],
            Some(Texture::DEPTH_FORMAT),
            &[wgpu::VertexBufferDescriptor {
                stride: 16 * 2,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttributeDescriptor {
                        offset: 0,
                        format: wgpu::VertexFormat::Float3,
                        shader_location: 0,
                    },
                    wgpu::VertexAttributeDescriptor {
                        offset: 16,
                        format: wgpu::VertexFormat::Float2,
                        shader_location: 1,
                    },
                    wgpu::VertexAttributeDescriptor {
                        offset: 16 + 8,
                        format: wgpu::VertexFormat::Float,
                        shader_location: 2,
                    },
                ],
            }],
            vk_shader_macros::include_glsl!("src/render/shaders/3d.vert"),
            vk_shader_macros::include_glsl!("src/render/shaders/3d.frag"),
            sample_count,
        );

        D3 {
            pipeline,
            depth_texture,
            normal_texture_binding,
            normal_texture_binding_layout,
        }
    }

    pub fn depth_stencil_attachement(&self) -> wgpu::RenderPassDepthStencilAttachmentDescriptor {
        wgpu::RenderPassDepthStencilAttachmentDescriptor {
            attachment: &self.depth_texture.view,
            depth_load_op: wgpu::LoadOp::Clear,
            depth_store_op: wgpu::StoreOp::Store,
            clear_depth: 1.0,
            stencil_load_op: wgpu::LoadOp::Clear,
            stencil_store_op: wgpu::StoreOp::Store,
            clear_stencil: 0,
        }
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        sc_desc: &wgpu::SwapChainDescriptor,
        sample_count: u32,
    ) {
        self.depth_texture =
            Texture::create_depth_texture(device, sc_desc, sample_count, Some("Depth texture"));
    }

    pub fn update_bindgroup(&mut self, device: &wgpu::Device, compute: &IRCompute) {
        self.normal_texture_binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.normal_texture_binding_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&compute.normal_texture.view),
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&compute.normal_texture.sampler),
                },
            ],
            label: Some("Normal texture sampled"),
        });
    }

    pub fn render<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        compute: &'a IRCompute,
        uniforms: &'a BoundBuffer<Uniforms>,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, compute.vertices(), 0, 0);
        pass.set_index_buffer(compute.indices(), 0, 0);
        pass.set_bind_group(0, &uniforms.bind_group(), &[]);
        pass.set_bind_group(1, &self.normal_texture_binding, &[]);
        pass.draw_indexed(0..compute.indices_count(), 0, 0..1);
    }
}
