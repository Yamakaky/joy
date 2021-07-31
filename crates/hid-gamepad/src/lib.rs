mod error;

use dualshock::DS4Driver;
use hid_gamepad_sys::{GamepadDevice, GamepadDriver};
use hidapi::{DeviceInfo, HidApi};

pub use hid_gamepad_sys as sys;

pub use error::*;
use joycon::{JoyCon, JoyconDriver};

pub fn open_gamepad(
    api: &HidApi,
    device_info: &DeviceInfo,
) -> Result<Option<Box<dyn GamepadDevice + 'static>>> {
    let mut drivers: Vec<Box<dyn GamepadDriver>> =
        vec![Box::new(DS4Driver), Box::new(JoyconDriver)];
    for driver in drivers.drain(..) {
        if let Some(device) = driver.init(api, device_info)? {
            return Ok(Some(device));
        }
    }
    Ok(None)
}

pub fn pote() {
    let api = HidApi::new().unwrap();
    let device_info = api.device_list().next().unwrap();
    let mut x = open_gamepad(&api, device_info).unwrap().unwrap();
    x.as_any().downcast_ref::<Box<JoyCon>>();
}
