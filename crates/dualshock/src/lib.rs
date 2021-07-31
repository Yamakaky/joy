use anyhow::Result;
use dualshock_sys::{
    input::InputReport, ConnectionType, DS4_REPORT_RATE, HID_PRODUCT_ID_NEW, HID_PRODUCT_ID_OLD,
    HID_VENDOR_ID,
};
use hid_gamepad_sys::{GamepadDevice, GamepadDriver, JoyKey, KeyStatus, Motion, Report};
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
        let b = &full.base.buttons;
        let rot = full.gyro.normalize();
        Ok(Report {
            left_joystick: full.base.left_stick.normalize(),
            right_joystick: full.base.right_stick.normalize(),
            motion: vec![Motion {
                acceleration: full.accel.normalize(),
                rotation_speed: rot.into(),
            }],
            keys: enum_map::enum_map! {
                JoyKey::Up => b.dpad().up().into(),
                JoyKey::Down => b.dpad().down().into(),
                JoyKey::Left => b.dpad().left().into(),
                JoyKey::Right=> b.dpad().right().into(),
                JoyKey::N => b.triangle().into(),
                JoyKey::S => b.cross().into(),
                JoyKey::E => b.circle().into(),
                JoyKey::W => b.square().into(),
                JoyKey::L=> b.l1().into(),
                JoyKey::R=> b.r1().into(),
                JoyKey::ZL => b.l2().into(),
                JoyKey::ZR => b.r2().into(),
                JoyKey::SL => KeyStatus::Released,
                JoyKey::SR => KeyStatus::Released,
                JoyKey::L3 => b.l3().into(),
                JoyKey::R3 => b.r3().into(),
                JoyKey::Minus => b.tpad().into(),
                JoyKey::Plus => b.options().into(),
                JoyKey::Capture => b.share().into(),
                JoyKey::Home => b.ps().into(),
            },
            frequency: DS4_REPORT_RATE,
        })
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
