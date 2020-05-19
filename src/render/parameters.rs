use super::JoyconCmd;
use joycon_sys::mcu::ir::*;
use std::sync::mpsc;

pub struct Parameters {
    resolution: Resolution,
    flip: Flip,
    denoise: bool,
}

impl Parameters {
    pub fn new() -> Self {
        Self {
            resolution: Resolution::R320x240,
            flip: Flip::Normal,
            denoise: true,
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
                        .send(JoyconCmd::SetResolution(self.resolution))
                        .unwrap();
                    true
                }
                VirtualKeyCode::F => {
                    use Flip::*;
                    self.flip = match self.flip {
                        Normal => Vertically,
                        Vertically => Horizontally,
                        Horizontally => Both,
                        Both => Normal,
                    };
                    thread_contact
                        .send(JoyconCmd::SetRegister(Register::flip(self.flip)))
                        .unwrap();
                    true
                }
                VirtualKeyCode::N => {
                    self.denoise = !self.denoise;
                    thread_contact
                        .send(JoyconCmd::SetRegister(Register::denoise(self.denoise)))
                        .unwrap();
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }
}
