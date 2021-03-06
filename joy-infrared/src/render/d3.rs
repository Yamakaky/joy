use super::{
    buffer::BoundBuffer,
    ir_compute::IRCompute,
    texture::Texture,
    uniforms::{Lights, Uniforms},
};
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
        lights: &BoundBuffer<Lights>,
        compute: &IRCompute,
        sc_desc: &wgpu::SwapChainDescriptor,
        sample_count: u32,
    ) -> Self {
        let depth_texture =
            Texture::create_depth_texture(&device, &sc_desc, sample_count, Some("Depth texture"));

        let normal_texture_binding_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            dimension: wgpu::TextureViewDimension::D2,
                            component_type: wgpu::TextureComponentType::Float,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                        count: None,
                    },
                ],
                label: Some("Normal texture 3D pipeline"),
            });
        let normal_texture_binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &normal_texture_binding_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&compute.normal_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&compute.normal_texture.sampler),
                },
            ],
            label: Some("Normal texture sampled"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[
                uniforms.bind_group_layout(),
                &normal_texture_binding_layout,
                lights.bind_group_layout(),
            ],
            push_constant_ranges: &[],
            label: Some("3D Pipeline Layout"),
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
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: true,
            }),
            stencil_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(0),
                store: true,
            }),
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
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&compute.normal_texture.view),
                },
                wgpu::BindGroupEntry {
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
        lights: &'a BoundBuffer<Lights>,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, compute.vertices().slice(..));
        pass.set_index_buffer(compute.indices().slice(..));
        pass.set_bind_group(0, &uniforms.bind_group(), &[]);
        pass.set_bind_group(1, &self.normal_texture_binding, &[]);
        pass.set_bind_group(2, &lights.bind_group(), &[]);
        pass.draw_indexed(0..compute.indices_count(), 0, 0..1);
    }
}
