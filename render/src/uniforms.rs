#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Uniforms {
    view_proj: cgmath::Matrix4<f32>,
    pub width: u32,
    pub height: u32,
}

impl Uniforms {
    pub fn new() -> Uniforms {
        use cgmath::SquareMatrix;
        Uniforms {
            view_proj: cgmath::Matrix4::identity(),
            width: 0,
            height: 0,
        }
    }

    pub fn update_view_proj(&mut self, camera: &crate::camera::Camera) {
        self.view_proj = camera.build_view_projection_matrix();
    }

    pub fn instance_count(&self) -> u32 {
        self.width * self.height
    }

    pub const fn layout() -> [wgpu::BindGroupLayoutEntry; 1] {
        [wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStage::VERTEX,
            ty: wgpu::BindingType::UniformBuffer { dynamic: false },
        }]
    }

    pub fn bindings<'a>(&self, uniform_buffer: &'a wgpu::Buffer) -> [wgpu::Binding<'a>; 1] {
        [wgpu::Binding {
            binding: 0,
            resource: wgpu::BindingResource::Buffer {
                buffer: uniform_buffer,
                range: 0..std::mem::size_of::<Self>() as wgpu::BufferAddress,
            },
        }]
    }
}

unsafe impl bytemuck::Zeroable for Uniforms {}
unsafe impl bytemuck::Pod for Uniforms {}
