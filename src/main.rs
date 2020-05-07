use hidapi::HidApi;
use joycon_sys::input::Vector3;

mod calibration;
mod hid;

fn main() -> anyhow::Result<()> {
    let api = HidApi::new()?;

    for device in api
        .device_list()
        .filter(|x| x.vendor_id() == joycon_sys::NINTENDO_VENDOR_ID)
    {
        let mut device = hid::JoyCon::new(device.open_device(&api)?, device.clone());
        println!("new dev {:?}", device);
        println!("info: {:?}", device.get_dev_info()?);

        device.set_imu_sens()?;
        device.enable_imu()?;
        device.set_standard_mode()?;
        device.set_player_light(joycon_sys::output::PlayerLights::new(
            true, false, false, true, false, false, false, false,
        ))?;

        dbg!(device.load_calibration()?);
        device.reset_calibration()?;

        let iter_count = 600;
        let mut rotation = Vector3::default();
        let mut image = image::ImageBuffer::new(iter_count, 128);
        for i in 0..iter_count {
            for y in 0..(device.max_raw_gyro / 256) {
                image.put_pixel(i, 127 - y as u32, image::Luma([128u8]));
            }
            device.max_raw_gyro = 0;
            image.put_pixel(i, 127 - 32, image::Luma([255u8]));
            image.put_pixel(i, 127 - 64, image::Luma([255u8]));
            for delta in &device.get_gyro_rot_delta(false)? {
                rotation += *delta;
            }

            if i % 60 == 0 {
                println!("{:?}", rotation);
                println!("{:?}", device.get_accel_delta_g(false)?[0]);
            }
        }
        image.save("D:\\gyro.png")?;
    }
    Ok(())
}
