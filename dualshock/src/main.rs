use std::time::{Duration, Instant};

use cgmath::{Deg, Euler, One, Quaternion};
use dualshock_sys::{input::InputReport, HID_PRODUCT_ID_BT, HID_PRODUCT_ID_USB, HID_VENDOR_ID};

fn main() -> anyhow::Result<()> {
    let hidapi = hidapi::HidApi::new()?;
    let device_info = hidapi
        .device_list()
        .filter(|d| {
            dbg!(d);
            d.vendor_id() == HID_VENDOR_ID
                && [HID_PRODUCT_ID_BT, HID_PRODUCT_ID_USB].contains(&d.product_id())
        })
        .next()
        .unwrap();
    let device = device_info.open_device(&hidapi)?;
    let mut now = Instant::now();
    let mut orientation = Quaternion::one();
    loop {
        let mut report = InputReport::new();
        let buffer = report.as_bytes_mut();
        let _nb_read = device.read(buffer)?;
        if device_info.product_id() == HID_PRODUCT_ID_BT {
            let report = report.complete().unwrap();
            orientation = orientation * Quaternion::from(report.full.gyro.delta());
            if now.elapsed() > Duration::from_millis(500) {
                let rot = Euler::from(orientation);
                dbg!(Deg::from(rot.x));
                now = Instant::now();
            }
        } else {
            let report = report.usb().unwrap();
            orientation = orientation * Quaternion::from(report.full.gyro.delta());
            if now.elapsed() > Duration::from_millis(500) {
                let rot = Euler::from(orientation);
                dbg!(Deg::from(rot.y));
                now = Instant::now();
            }
        }
    }
}
