pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn new(sc_desc: &wgpu::SwapChainDescriptor) -> Camera {
        let mut camera = Camera {
            eye: (2., 1., 10.).into(),
            target: (2., 1., 0.).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: sc_desc.width as f32 / sc_desc.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 200.0,
        };
        camera.update_aspect(sc_desc.width, sc_desc.height);
        camera
    }

    pub fn update_aspect(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32
    }

    pub fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }

    pub fn input(&mut self, event: &winit::event::WindowEvent) -> bool {
        use winit::event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent};
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => match keycode {
                VirtualKeyCode::Z => {
                    self.eye += (0., 1., 0.).into();
                    true
                }
                VirtualKeyCode::Q => {
                    self.eye -= (1., 0., 0.).into();
                    true
                }
                VirtualKeyCode::S => {
                    self.eye -= (0., 1., 0.).into();
                    true
                }
                VirtualKeyCode::D => {
                    self.eye += (1., 0., 0.).into();
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }
}

// from https://sotrh.github.io/learn-wgpu/beginner/tutorial6-uniforms/#a-perspective-camera
#[cfg_attr(rustfmt, rustfmt_skip)]
 const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);
