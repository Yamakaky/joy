use crate::render::camera::Camera;
use bytemuck::{Pod, Zeroable};
use cgmath::{prelude::*, vec3, vec4, Deg, Matrix3, Matrix4, Quaternion, Vector3, Vector4};

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

#[repr(C, align(16))]
#[derive(Debug, Copy, Clone)]
pub struct Lights {
    count: u32,
    _pad: [u32; 3],
    lights: [Light; Self::MAX as usize],
}
unsafe impl Zeroable for Lights {}
unsafe impl Pod for Lights {}

impl Lights {
    const MAX: u32 = 10;

    pub fn lights() -> Self {
        Self::zeroed()
            .push(Light {
                position: vec4(0., 0., 0., 1.0),
                ambient: vec3(0., 1., 0.) * 0.05,
                diffuse: vec3(0., 1., 0.) * 0.8,
                specular: vec3(0., 1., 0.),
                constant: 1.0,
                linear: 0.7,
                quadratic: 1.8,
                ..Light::default()
            })
            .push(Light {
                position: vec4(0.2, 1., -0.2, 0.0),
                ambient: vec3(0.05, 0.05, 0.05),
                diffuse: vec3(0.4, 0.4, 0.4),
                specular: vec3(0.5, 0.5, 0.5),
                constant: 1.0,
                linear: 0.7,
                quadratic: 1.8,
                ..Light::default()
            })
    }

    fn push(mut self, light: Light) -> Self {
        assert_ne!(Self::MAX, self.count);
        self.lights[self.count as usize] = light;
        self.count += 1;
        self
    }
}

impl Default for Lights {
    fn default() -> Self {
        Self::zeroed()
    }
}

#[repr(C, align(16))]
#[derive(Debug, Copy, Clone)]
pub struct Light {
    position: Vector4<f32>,
    ambient: Vector3<f32>,
    _pad1: u32,
    diffuse: Vector3<f32>,
    _pad2: u32,
    specular: Vector3<f32>,
    constant: f32,
    linear: f32,
    quadratic: f32,
}
unsafe impl Zeroable for Light {}
unsafe impl Pod for Light {}

impl Default for Light {
    fn default() -> Self {
        Self::zeroed()
    }
}
