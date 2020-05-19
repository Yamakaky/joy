use cgmath::prelude::*;
use cgmath::Vector3;
use std::collections::HashMap;
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
        let mut vertices = vec![];
        let mut positions: HashMap<_, Vec<Vector3<f32>>> = HashMap::new();
        for x in 0..(width - 1) {
            for y in 0..(height - 1) {
                {
                    let i = x + y * width;
                    let a = points[i];
                    let b = points[i + 1];
                    let c = points[i + width];
                    let normal = (b - a).cross(c - a).normalize();
                    positions
                        .entry((a.x as u32, a.y as u32))
                        .or_default()
                        .push(normal);
                    positions
                        .entry((b.x as u32, b.y as u32))
                        .or_default()
                        .push(normal);
                    positions
                        .entry((c.x as u32, c.y as u32))
                        .or_default()
                        .push(normal);
                    vertices.push(Vertex {
                        position: a,
                        normal,
                    });
                    vertices.push(Vertex {
                        position: b,
                        normal,
                    });
                    vertices.push(Vertex {
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
                    positions
                        .entry((a.x as u32, a.y as u32))
                        .or_default()
                        .push(normal);
                    positions
                        .entry((b.x as u32, b.y as u32))
                        .or_default()
                        .push(normal);
                    positions
                        .entry((c.x as u32, c.y as u32))
                        .or_default()
                        .push(normal);
                    vertices.push(Vertex {
                        position: a,
                        normal,
                    });
                    vertices.push(Vertex {
                        position: b,
                        normal,
                    });
                    vertices.push(Vertex {
                        position: c,
                        normal,
                    });
                }
            }
        }
        for vertex in &mut vertices {
            let pos = (vertex.position.x as u32, vertex.position.y as u32);
            let normals = &positions[&pos];
            assert!(normals.len() > 0, "{:?}", pos);
            let average_normal =
                normals.iter().fold(Vector3::zero(), std::ops::Add::add) / normals.len() as f32;
            vertex.normal = average_normal;
        }
        vertices
    }
}
