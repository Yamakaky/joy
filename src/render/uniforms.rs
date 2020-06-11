use crate::render::camera::Camera;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Uniforms {
    ir_proj: cgmath::Matrix4<f32>,
    ir_rotation: cgmath::Matrix4<f32>,
    view_proj: cgmath::Matrix4<f32>,
}

unsafe impl Zeroable for Uniforms {}
unsafe impl Pod for Uniforms {}

impl Uniforms {
    pub fn new() -> Uniforms {
        use cgmath::SquareMatrix;
        let ir_proj = cgmath::perspective(cgmath::Deg(110.), 3. / 4., 0.1, 1.)
            .invert()
            .unwrap();
        Uniforms {
            ir_proj,
            ir_rotation: cgmath::Matrix4::identity(),
            view_proj: cgmath::Matrix4::identity(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix();
    }

    pub fn set_ir_rotation(&mut self, rotation: cgmath::Quaternion<f64>) {
        self.ir_rotation = cgmath::Matrix4::from(rotation).cast().unwrap();
    }
}

impl Default for Uniforms {
    fn default() -> Self {
        Self::new()
    }
}
