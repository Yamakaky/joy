use super::JoyconCmd;
use joycon_sys::mcu::ir::*;
use std::sync::mpsc;

pub struct Parameters {
    resolution: Resolution,
}

impl Parameters {
    pub fn new() -> Self {
        Self {
            resolution: Resolution::R320x240,
        }
    }
    pub fn input(
        &mut self,
        event: &winit::event::WindowEvent,
        thread_contact: &mpsc::Sender<JoyconCmd>,
    ) -> bool {
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
                VirtualKeyCode::R => {
                    use Resolution::*;
                    self.resolution = match self.resolution {
                        R320x240 => R160x120,
                        R160x120 => R80x60,
                        R80x60 => R40x30,
                        R40x30 => R320x240,
                    };
                    thread_contact
                        .send(JoyconCmd::SetRegister(Register::resolution(
                            self.resolution,
                        )))
                        .unwrap();
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }
}
