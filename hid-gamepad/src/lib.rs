mod error;

use dualshock::DS4Driver;
use hid_gamepad_sys::{GamepadDevice, GamepadDriver};
use hidapi::{DeviceInfo, HidApi};

pub use error::*;
use joycon::JoyconDriver;

pub fn open_gamepad(
    api: &HidApi,
    device_info: DeviceInfo,
) -> Result<Option<Box<dyn GamepadDevice>>> {
    let mut drivers: Vec<Box<dyn GamepadDriver>> =
        vec![Box::new(DS4Driver), Box::new(JoyconDriver)];
    for driver in drivers.drain(..) {
        if let Some(device) = driver.init(api, &device_info)? {
            return Ok(Some(device));
        }
    }
    Ok(None)
}
