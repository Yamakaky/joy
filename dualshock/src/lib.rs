use anyhow::Result;
use dualshock_sys::{
    input::InputReport, ConnectionType, DS4_REPORT_RATE, HID_PRODUCT_ID_NEW, HID_PRODUCT_ID_OLD,
    HID_VENDOR_ID,
};
use hid_gamepad_sys::{GamepadDevice, GamepadDriver, Motion, Report};
use hidapi::{HidApi, HidDevice};

pub struct DS4Driver;

pub struct DS4 {
    device: HidDevice,
}

impl GamepadDriver for DS4Driver {
    fn init(
        &self,
        api: &HidApi,
        device_info: &hidapi::DeviceInfo,
    ) -> Result<Option<Box<dyn GamepadDevice>>> {
        if device_info.vendor_id() == HID_VENDOR_ID
            && [HID_PRODUCT_ID_OLD, HID_PRODUCT_ID_NEW].contains(&device_info.product_id())
        {
            Ok(Some(Box::new(DS4 {
                device: device_info.open_device(&api)?,
            })))
        } else {
            Ok(None)
        }
    }
}

impl GamepadDevice for DS4 {
    fn recv(&mut self) -> Result<Report> {
        let mut report = InputReport::new();
        let buffer = report.as_bytes_mut();
        let nb_read = self.device.read(buffer)?;
        let full = match InputReport::conn_type(nb_read) {
            ConnectionType::Bluetooth => &report.bt_full().unwrap().full,
            ConnectionType::USB => &report.usb_full().unwrap().full,
        };
        let mut out = Report::new();
        out.left_joystick = full.base.left_stick.normalize();
        out.right_joystick = full.base.right_stick.normalize();
        out.frequency = DS4_REPORT_RATE;
        out.motion = vec![Motion {
            acceleration: full.accel.normalize(),
            rotation_speed: full.gyro.normalize(),
        }];
        Ok(out)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
