use std::time::{Duration, Instant};

use cgmath::{Deg, Euler, One, Quaternion};
use dualshock_sys::{input::InputReport, HID_PRODUCT_ID_BT, HID_PRODUCT_ID_USB, HID_VENDOR_ID};

fn main() -> anyhow::Result<()> {
    let hidapi = hidapi::HidApi::new()?;
    let device = hidapi
        .device_list()
        .filter(|d| {
            dbg!(d);
            d.vendor_id() == HID_VENDOR_ID
                && [HID_PRODUCT_ID_BT, HID_PRODUCT_ID_USB].contains(&d.product_id())
        })
        .next()
        .unwrap();
    let device = device.open_device(&hidapi)?;
    let mut now = Instant::now();
    let mut orientation = Quaternion::one();
    loop {
        let mut report = InputReport::new();
        let buffer = report.as_bytes_mut();
        let _nb_read = device.read(buffer)?;
        if let Some(report) = report.complete() {
            let gyro = report.gyro.val();
            let dt = 1. / 250.;
            orientation = orientation
                * Quaternion::from(Euler::new(
                    Deg(gyro.y as f64 * dt * (2000.0 / 32767.0)),
                    Deg(gyro.z as f64 * dt * (2000.0 / 32767.0)),
                    Deg(gyro.x as f64 * dt * (2000.0 / 32767.0)),
                ));
            if now.elapsed() > Duration::from_millis(500) {
                let rot = Euler::from(orientation);
                dbg!(Deg::from(rot.x));
                now = Instant::now();
            }
        } else {
            if now.elapsed() > Duration::from_millis(500) {
                //dbg!(report.as_bytes_mut());
                now = Instant::now();
            }
        }
    }
}
