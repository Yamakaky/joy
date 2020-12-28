mod calibration;
mod hid;
mod image;
mod imu_handler;

pub use crate::image::*;
use anyhow::Result;
pub use calibration::*;
use cgmath::{Deg, Euler};
pub use hid::*;
use hid_gamepad_sys::{GamepadDevice, GamepadDriver, Motion};
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
            let mut joycon = JoyCon::new(device_info.open_device(&api)?, device_info.clone())?;
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
        let mut out = Self::new(IMU_SAMPLES_PER_SECOND);
        out.left_joystick = report.left_stick;
        out.right_joystick = report.right_stick;
        out.motion = report
            .imu
            .unwrap()
            .iter()
            .map(|x| Motion {
                acceleration: x.accel,
                rotation_speed: Euler::new(Deg(x.gyro.y), Deg(x.gyro.z), Deg(x.gyro.x)),
            })
            .collect();
        out
    }
}
