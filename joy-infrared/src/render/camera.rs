use cgmath::{prelude::*, Deg, Euler, Matrix4, Quaternion, Vector3};
use iced_wgpu::wgpu;

use iced_winit::winit::{
    self,
    event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent},
};

pub struct Camera {
    eye: Vector3<f32>,
    pitch: Deg<f32>,
    yaw: Deg<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,

    controller: CameraController,
}

impl Camera {
    pub fn new(sc_desc: &wgpu::SwapChainDescriptor) -> Camera {
        let mut camera = Camera {
            eye: (0., 0., 0.).into(),
            pitch: Deg(0.),
            yaw: Deg(180.),
            aspect: sc_desc.width as f32 / sc_desc.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 10.0,
            controller: CameraController::default(),
        };
        camera.update_aspect(sc_desc.width, sc_desc.height);
        camera
    }

    pub fn update_aspect(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32
    }

    fn eye_rot(&self) -> Quaternion<f32> {
        Quaternion::from(Euler::new(self.pitch, self.yaw, Deg(0.)))
    }

    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        let view = Matrix4::from(self.eye_rot()) * Matrix4::from_translation(self.eye);
        OPENGL_TO_WGPU_MATRIX * proj * view
    }

    pub fn input(&mut self, event: &winit::event::WindowEvent) -> bool {
        self.controller.input(event)
    }

    pub fn mouse_move(&mut self, delta: (f64, f64)) {
        self.controller.mouse_move(delta);
    }

    pub fn update(&mut self, dt: std::time::Duration) {
        let sens_dpu = 1.;
        self.pitch = Deg({
            let pitch = self.pitch.0 + self.controller.mouse_delta.1 as f32 * sens_dpu;
            if pitch > 90. {
                90.
            } else if pitch < -90. {
                -90.
            } else {
                pitch
            }
        });
        self.yaw += Deg(self.controller.mouse_delta.0 as f32 * sens_dpu);
        self.yaw = self.yaw.normalize();
        self.controller.mouse_delta = (0., 0.);

        let c = &self.controller;
        let mut sum = Vector3::zero();
        if c.right {
            sum -= Vector3::unit_x();
        }
        if c.left {
            sum += Vector3::unit_x();
        }
        if c.up {
            sum += Vector3::unit_y();
        }
        if c.down {
            sum -= Vector3::unit_y();
        }
        if c.forward {
            sum += Vector3::unit_z();
        }
        if c.backward {
            sum -= Vector3::unit_z();
        }
        if sum != Vector3::zero() {
            let speed_ups = 2.;
            self.eye += self.eye_rot().invert().rotate_vector(sum.normalize())
                * speed_ups
                * dt.as_millis() as f32
                / 1000.;
        }
    }
}

#[derive(Default)]
struct CameraController {
    mouse_delta: (f64, f64),
    up: bool,
    down: bool,
    right: bool,
    left: bool,
    backward: bool,
    forward: bool,
}

impl CameraController {
    pub fn mouse_move(&mut self, delta: (f64, f64)) {
        self.mouse_delta = delta;
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => {
                let pressed = *state == ElementState::Pressed;
                match keycode {
                    VirtualKeyCode::Z => {
                        self.forward = pressed;
                        true
                    }
                    VirtualKeyCode::Q => {
                        self.left = pressed;
                        true
                    }
                    VirtualKeyCode::S => {
                        self.backward = pressed;
                        true
                    }
                    VirtualKeyCode::D => {
                        self.right = pressed;
                        true
                    }
                    VirtualKeyCode::A => {
                        self.up = pressed;
                        true
                    }
                    VirtualKeyCode::E => {
                        self.down = pressed;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }
}

// from https://sotrh.github.io/learn-wgpu/beginner/tutorial6-uniforms/#a-perspective-camera
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);
