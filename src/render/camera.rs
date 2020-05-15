pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
    pub ortho: bool,
}

impl Camera {
    pub fn new(sc_desc: &wgpu::SwapChainDescriptor) -> Camera {
        let mut camera = Camera {
            eye: (120., 120., 20.).into(),
            target: (120., 120., -50.).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: sc_desc.width as f32 / sc_desc.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 10000.0,
            ortho: false,
        };
        camera.update_aspect(sc_desc.width, sc_desc.height);
        camera
    }

    pub fn update_aspect(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32
    }

    pub fn build_view_projection_matrix(&self, width: u32, height: u32) -> cgmath::Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX
            * if self.ortho {
                cgmath::Ortho {
                    left: 0.,
                    right: height as f32,
                    bottom: 0.,
                    top: width as f32,
                    far: 256.,
                    near: -1.,
                }
                .into()
            } else {
                let proj =
                    cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
                let view = cgmath::Matrix4::look_at(self.eye, self.target, self.up);
                proj * view
            }
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
                VirtualKeyCode::Space => {
                    self.ortho = !self.ortho;
                    true
                }
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
                VirtualKeyCode::A => {
                    self.eye -= (0., 0., 1.).into();
                    true
                }
                VirtualKeyCode::E => {
                    self.eye += (0., 0., 1.).into();
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
