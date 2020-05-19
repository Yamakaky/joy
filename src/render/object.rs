use cgmath::InnerSpace;
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
    pub const BUF_SIZE: wgpu::BufferAddress = size_of::<Self>() as u64 * 320 * 240 * 6;

    pub fn descriptor() -> wgpu::VertexBufferDescriptor<'static> {
        assert_eq!(size_of::<Self>(), 3 * 4 * 2);
        wgpu::VertexBufferDescriptor {
            stride: size_of::<Self>() as u64,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float3,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: size_of::<Vector3<f32>>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float3,
                },
            ],
        }
    }

    pub fn from_ir(buffer: &[u8], width: u32, height: u32) -> Vec<Vertex> {
        let width = width as usize;
        let height = height as usize;
        let points: Vec<Vector3<f32>> = buffer
            .iter()
            .enumerate()
            .map(|(i, z)| {
                let x = (i % width) as f32;
                let y = (i / width) as f32;
                Vector3::new(height as f32 - y, width as f32 - x, (*z as f32) - 255.)
            })
            .collect();
        let mut out = vec![];
        for x in 0..(width - 1) {
            for y in 0..(height - 1) {
                {
                    let i = x + y * width;
                    let a = points[i];
                    let b = points[i + 1];
                    let c = points[i + width];
                    let normal = (b - a).cross(c - a).normalize();
                    assert!(normal != Vector3::new(0., 0., 0.));
                    out.push(Vertex {
                        position: a,
                        normal,
                    });
                    out.push(Vertex {
                        position: b,
                        normal,
                    });
                    out.push(Vertex {
                        position: c,
                        normal,
                    });
                }

                {
                    let i = (x + 1) + (y + 1) * width;
                    let a = points[i];
                    let b = points[i - 1];
                    let c = points[i - width];
                    let normal = (b - a).cross(c - a).normalize();
                    out.push(Vertex {
                        position: a,
                        normal,
                    });
                    out.push(Vertex {
                        position: b,
                        normal,
                    });
                    out.push(Vertex {
                        position: c,
                        normal,
                    });
                }
            }
        }
        out
    }
}
