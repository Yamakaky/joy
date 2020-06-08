use super::ir_compute::IRCompute;
use super::texture::Texture;
use super::uniforms::UniformHandler;
use iced_wgpu::wgpu;

pub struct D3 {
    pipeline: wgpu::RenderPipeline,
    depth_texture: Texture,
}

impl D3 {
    pub fn new(
        device: &wgpu::Device,
        uniforms: &UniformHandler,
        sc_desc: &wgpu::SwapChainDescriptor,
        sample_count: u32,
    ) -> Self {
        let depth_texture =
            Texture::create_depth_texture(&device, &sc_desc, sample_count, Some("Depth texture"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[uniforms.bind_group_layout()],
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
                        format: wgpu::VertexFormat::Float3,
                        shader_location: 1,
                    },
                    wgpu::VertexAttributeDescriptor {
                        offset: 16 + 12,
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

    pub fn render<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        compute: &'a IRCompute,
        uniforms: &'a UniformHandler,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, compute.vertices(), 0, 0);
        pass.set_index_buffer(compute.indices(), 0, 0);
        pass.set_bind_group(0, &uniforms.bind_group(), &[]);
        pass.draw_indexed(0..compute.indices_count(), 0, 0..1);
    }
}
