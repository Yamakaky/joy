use crate::JoyCon;
use anyhow;
use hidapi::{DeviceInfo, HidApi, HidError};
use joycon_sys::{
    input::WhichController, JOYCON_L_BT, JOYCON_R_BT, NINTENDO_VENDOR_ID, PRO_CONTROLLER,
};
use thiserror::Error;

/// Helper for creating [`JoyCon`]s.
///
/// The main purpose of this struct is to be returned by [`scan_for_joycons`].
pub struct DetectedJoyConInfo<'a> {
    api: &'a HidApi,
    device_info: &'a DeviceInfo,
}

/// Helper for [`DetectedJoyConInfo::connect`].
#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("failed to connect to the device")]
    Hid(HidError),
    #[error("failed to create a new JoyCon")]
    JoyCon(anyhow::Error),
}

impl DetectedJoyConInfo<'_> {
    pub fn device_type(&self) -> anyhow::Result<WhichController> {
        WhichController::from_product_id(self.device_info.product_id())
    }

    pub fn connect(&self) -> Result<JoyCon, ConnectionError> {
        match self.device_info.open_device(self.api) {
            Ok(hid_device) => match JoyCon::new(hid_device, self.device_info.clone()) {
                Ok(joycon) => Ok(joycon),
                Err(anyhow_error) => Err(ConnectionError::JoyCon(anyhow_error)),
            },
            Err(hid_error) => Err(ConnectionError::Hid(hid_error)),
        }
    }
}

/// An easy way to create [`JoyCon`]s.
///
/// Use this function to find all of the Joy-Cons (or Pro Controllers) connected
/// to the system. you can then connect to them with
/// [`DetectedJoyConInfo::connect()`].
///
/// ```
/// use joycon::{hidapi::HidApi, joycon_sys::input::WhichController, scan_for_joycons};
///
/// fn main() {
///     let api = HidApi::new().expect("Failed to create new HipApi.");
///     for joy_con_info in scan_for_joycons(&api) {
///         let device_type = joy_con_info
///             .device_type()
///             .expect("scan_for_joycons returned info for something that isn’t a Joy-Con or a Pro Controller. This should never happen.");
///
///         if device_type == WhichController::RightJoyCon {
///             match joy_con_info.connect() {
///                 Ok(joy_con) => {
///                     println!("Successfully connected.");
///                     assert!(joy_con.supports_ir());
///                 }
///                 Err(_) => println!("Failed to connect."),
///             }
///         }
///     }
/// }
/// ```
pub fn scan_for_joycons(api: &HidApi) -> Vec<DetectedJoyConInfo> {
    let mut return_value: Vec<DetectedJoyConInfo> = Vec::new();
    for device in api.device_list() {
        if device.vendor_id() == NINTENDO_VENDOR_ID
            && [JOYCON_L_BT, JOYCON_R_BT, PRO_CONTROLLER].contains(&device.product_id())
        {
            return_value.push(DetectedJoyConInfo {
                api: api,
                device_info: device,
            });
        }
    }
    return_value
}
