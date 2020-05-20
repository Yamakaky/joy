use super::JoyconCmd;
use joycon_sys::mcu::ir::*;
use std::sync::mpsc;

pub struct Parameters {
    resolution: Resolution,
    flip: Flip,
    denoise: bool,
    leds: Leds,
    ext_light_filter: ExternalLightFilter,
}

impl Parameters {
    pub fn new() -> Self {
        Self {
            resolution: Resolution::R320x240,
            flip: Flip::Normal,
            denoise: true,
            leds: Leds(0),
            ext_light_filter: ExternalLightFilter::X1,
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
                VirtualKeyCode::X => {
                    self.ext_light_filter = if self.ext_light_filter == ExternalLightFilter::X1 {
                        ExternalLightFilter::Off
                    } else {
                        ExternalLightFilter::X1
                    };
                    thread_contact
                        .send(JoyconCmd::SetRegister(Register::external_light_filter(
                            self.ext_light_filter,
                        )))
                        .unwrap();
                    true
                }
                VirtualKeyCode::L => {
                    let (far, near) = match (
                        self.leds.disable_far_narrow12(),
                        self.leds.disable_near_wide34(),
                    ) {
                        (false, false) => (true, false),
                        (true, false) => (true, true),
                        (true, true) => (false, true),
                        (false, true) => (false, false),
                    };
                    self.leds.set_disable_far_narrow12(far);
                    self.leds.set_disable_near_wide34(near);
                    dbg!(self.leds);
                    thread_contact
                        .send(JoyconCmd::SetRegister(Register::ir_leds(self.leds)))
                        .unwrap();
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }
}
