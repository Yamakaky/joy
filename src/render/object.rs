use cgmath::Vector3;
use std::mem::size_of;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Vertex {
    position: Vector3<f32>,
    normal: Vector3<f32>,
}
unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

impl Vertex {
    pub fn descriptor() -> wgpu::VertexBufferDescriptor<'static> {
        assert_eq!(size_of::<Self>(), 3 * 4 * 2);
        wgpu::VertexBufferDescriptor {
            stride: size_of::<Self>() as u64,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float3, 1 => Float3],
        }
    }
}
