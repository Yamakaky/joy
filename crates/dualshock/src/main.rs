use std::time::{Duration, Instant};

use cgmath::{Deg, Euler, One, Quaternion};
use dualshock_sys::{
    input::InputReport, ConnectionType, DS4_REPORT_DT, HID_PRODUCT_ID_NEW, HID_PRODUCT_ID_OLD,
    HID_VENDOR_ID,
};

fn main() -> anyhow::Result<()> {
    let hidapi = hidapi::HidApi::new()?;
    let device_info = hidapi
        .device_list()
        .filter(|d| {
            d.vendor_id() == HID_VENDOR_ID
                && [HID_PRODUCT_ID_OLD, HID_PRODUCT_ID_NEW].contains(&d.product_id())
        })
        .next()
        .unwrap();
    let device = device_info.open_device(&hidapi)?;

    let mut report = InputReport::new();
    let buffer = report.as_bytes_mut();
    let nb_read = device.read(buffer)?;
    let conn_type = InputReport::conn_type(nb_read);

    let mut now = Instant::now();
    let mut orientation = Quaternion::one();
    loop {
        let mut report = InputReport::new();
        let buffer = report.as_bytes_mut();
        let _nb_read = device.read(buffer)?;
        let gyro_speed = match conn_type {
            ConnectionType::Bluetooth => {
                let report = report.bt_full().unwrap();
                report.full.gyro.normalize()
            }
            ConnectionType::USB => {
                let report = report.usb_full().unwrap();
                report.full.gyro.normalize()
            }
        };

        let delta = Euler::new(
            gyro_speed.x * DS4_REPORT_DT,
            gyro_speed.y * DS4_REPORT_DT,
            gyro_speed.z * DS4_REPORT_DT,
        );

        orientation = orientation * Quaternion::from(delta);
        if now.elapsed() > Duration::from_millis(500) {
            let rot = Euler::from(orientation);
            dbg!(Deg::from(rot.x));
            now = Instant::now();
        }
    }
}
