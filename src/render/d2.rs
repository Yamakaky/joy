use cgmath::vec2;
use iced_wgpu::wgpu;

#[repr(C)]
#[derive(Copy, Clone)]
struct Vertex2D {
    position: cgmath::Vector2<f32>,
    uv: cgmath::Vector2<f32>,
}
unsafe impl bytemuck::Pod for Vertex2D {}
unsafe impl bytemuck::Zeroable for Vertex2D {}

pub struct D2 {
    index_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
}

impl D2 {
    pub fn new(
        device: &wgpu::Device,
        texture_binding_layout: &wgpu::BindGroupLayout,
        sample_count: u32,
    ) -> Self {
        let vertices = &[
            Vertex2D {
                position: vec2(0.5, -1.),
                uv: vec2(0., 1.),
            },
            Vertex2D {
                position: vec2(1., -1.),
                uv: vec2(1., 1.),
            },
            Vertex2D {
                position: vec2(0.5, 0.),
                uv: vec2(0., 0.),
            },
            Vertex2D {
                position: vec2(1., 0.),
                uv: vec2(1., 0.),
            },
        ];
        let vertex_buffer = device
            .create_buffer_with_data(bytemuck::cast_slice(vertices), wgpu::BufferUsage::VERTEX);
        let indices: &[u32] = &[0, 1, 2, 2, 1, 3];
        let index_buffer =
            device.create_buffer_with_data(bytemuck::cast_slice(indices), wgpu::BufferUsage::INDEX);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[texture_binding_layout],
        });

        let pipeline = super::create_render_pipeline(
            &device,
            &pipeline_layout,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            None,
            &[wgpu::VertexBufferDescriptor {
                stride: std::mem::size_of::<Vertex2D>() as wgpu::BufferAddress,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float2, 1 => Float2],
            }],
            vk_shader_macros::include_glsl!("src/render/shaders/2d.vert", kind: vert),
            vk_shader_macros::include_glsl!("src/render/shaders/2d.frag", kind: frag),
            sample_count,
        );
        D2 {
            vertex_buffer,
            index_buffer,
            pipeline,
        }
    }

    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, texture: &'a wgpu::BindGroup) {
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, &self.vertex_buffer, 0, 0);
        pass.set_index_buffer(&self.index_buffer, 0, 0);
        pass.set_bind_group(0, texture, &[]);
        pass.draw_indexed(0..6, 0, 0..1);
    }
}
