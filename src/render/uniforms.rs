use crate::render::buffer::Staged;
use crate::render::camera::Camera;
use iced_wgpu::wgpu;

pub struct UniformHandler {
    uniforms: Uniforms,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    dirty: bool,
}

impl UniformHandler {
    pub fn new(device: &wgpu::Device) -> Self {
        let uniforms = Uniforms::new();
        let raw_uniforms = bytemuck::bytes_of(&uniforms);
        let buffer = device.create_buffer_with_data(
            raw_uniforms,
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        );
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::COMPUTE,
                ty: wgpu::BindingType::UniformBuffer { dynamic: false },
            }],
            label: Some("Uniform bind group layout"),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer {
                    buffer: &buffer,
                    range: 0..raw_uniforms.len() as wgpu::BufferAddress,
                },
            }],
            label: Some("Uniform bind group"),
        });
        Self {
            uniforms,
            buffer,
            bind_group,
            bind_group_layout,
            dirty: true,
        }
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn set_ir_rotation(&mut self, rotation: cgmath::Quaternion<f64>) {
        self.uniforms.ir_rotation = cgmath::Matrix4::from(rotation).cast().unwrap();
        self.dirty = true;
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.uniforms.update_view_proj(camera);
        self.dirty = true;
    }

    pub fn upload(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder) {
        if self.dirty {
            self.buffer.update(device, encoder, &[self.uniforms]);
            self.dirty = false;
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct Uniforms {
    ir_proj: cgmath::Matrix4<f32>,
    ir_rotation: cgmath::Matrix4<f32>,
    view_proj: cgmath::Matrix4<f32>,
}

impl Uniforms {
    fn new() -> Uniforms {
        use cgmath::SquareMatrix;
        let ir_proj = cgmath::perspective(cgmath::Deg(95.), 3. / 4., 0.1, 1.)
            .invert()
            .unwrap();
        Uniforms {
            ir_proj,
            ir_rotation: cgmath::Matrix4::identity(),
            view_proj: cgmath::Matrix4::identity(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix();
    }
}

unsafe impl bytemuck::Zeroable for Uniforms {}
unsafe impl bytemuck::Pod for Uniforms {}
