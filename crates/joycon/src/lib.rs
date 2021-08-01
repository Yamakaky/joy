mod calibration;
mod hid;
mod image;
mod imu_handler;

pub use crate::image::*;
use anyhow::Result;
pub use calibration::*;
use cgmath::vec3;
pub use hid::*;
use hid_gamepad_sys::{GamepadDevice, GamepadDriver, JoyKey, Motion};
use hidapi::HidApi;
pub use imu_handler::IMU;
pub use joycon_sys;

pub use hidapi;
use joycon_sys::{imu::IMU_SAMPLES_PER_SECOND, NINTENDO_VENDOR_ID};

pub struct JoyconDriver;

impl GamepadDriver for JoyconDriver {
    fn init(
        &self,
        api: &HidApi,
        device_info: &hidapi::DeviceInfo,
    ) -> Result<Option<Box<dyn GamepadDevice>>> {
        if device_info.vendor_id() == NINTENDO_VENDOR_ID {
            let mut joycon = JoyCon::new(device_info.open_device(api)?, device_info.clone())?;
            joycon.enable_imu()?;
            joycon.load_calibration()?;
            Ok(Some(Box::new(joycon)))
        } else {
            Ok(None)
        }
    }
}

impl GamepadDevice for JoyCon {
    fn recv(&mut self) -> Result<hid_gamepad_sys::Report> {
        Ok(self.tick()?.into())
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl From<Report> for hid_gamepad_sys::Report {
    fn from(report: Report) -> Self {
        let b = &report.buttons;
        Self {
            left_joystick: report.left_stick,
            right_joystick: report.right_stick,
            motion: report
                .imu
                .unwrap()
                .iter()
                .map(|x| Motion {
                    acceleration: vec3(-x.accel.y, x.accel.z, x.accel.x).into(),
                    rotation_speed: vec3(x.gyro.y, -x.gyro.z, -x.gyro.x).into(),
                })
                .collect(),
            keys: enum_map::enum_map! {
                JoyKey::Up => b.left.up().into(),
                JoyKey::Down => b.left.down().into(),
                JoyKey::Left => b.left.left().into(),
                JoyKey::Right=> b.left.right().into(),
                JoyKey::N => b.right.x().into(),
                JoyKey::S => b.right.b().into(),
                JoyKey::E => b.right.a().into(),
                JoyKey::W => b.right.y().into(),
                JoyKey::L=> b.left.l().into(),
                JoyKey::R=> b.right.r().into(),
                JoyKey::ZL => b.left.zl().into(),
                JoyKey::ZR => b.right.zr().into(),
                JoyKey::SL => (b.left.sl() | b.right.sl()).into(),
                JoyKey::SR => (b.left.sr() | b.right.sr()).into(),
                JoyKey::L3 => b.middle.lstick().into(),
                JoyKey::R3 => b.middle.rstick().into(),
                JoyKey::Minus => b.middle.minus().into(),
                JoyKey::Plus => b.middle.plus().into(),
                JoyKey::Capture => b.middle.capture().into(),
                JoyKey::Home => b.middle.home().into(),
            },
            frequency: IMU_SAMPLES_PER_SECOND,
        }
    }
}
