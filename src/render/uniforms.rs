use crate::render::camera::Camera;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Uniforms {
    ir_proj: cgmath::Matrix4<f32>,
    pub ir_rotation: cgmath::Matrix4<f32>,
    view_proj: cgmath::Matrix4<f32>,
    pub width: u32,
    pub height: u32,
}

impl Uniforms {
    pub fn new() -> Uniforms {
        use cgmath::SquareMatrix;
        let ir_proj = cgmath::perspective(cgmath::Deg(95.), 3. / 4., 0.1, 1.)
            .invert()
            .unwrap();
        Uniforms {
            ir_proj,
            ir_rotation: cgmath::Matrix4::identity(),
            view_proj: cgmath::Matrix4::identity(),
            width: 0,
            height: 0,
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix();
    }

    pub fn layout() -> [wgpu::BindGroupLayoutEntry; 1] {
        [wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::COMPUTE,
            ty: wgpu::BindingType::UniformBuffer { dynamic: false },
        }]
    }

    pub fn bindings<'a>(&self, uniform_buffer: &'a wgpu::Buffer) -> [wgpu::Binding<'a>; 1] {
        [wgpu::Binding {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(uniform_buffer.slice(..)),
        }]
    }
}

unsafe impl bytemuck::Zeroable for Uniforms {}
unsafe impl bytemuck::Pod for Uniforms {}
