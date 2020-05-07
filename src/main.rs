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

        device.enable_imu()?;
        device.set_standard_mode()?;
        device.set_player_light(joycon_sys::output::PlayerLights::new(
            true, false, false, true, false, false, false, false,
        ))?;

        device.load_calibration()?;
        device.reset_calibration()?;

        let mut rotation = Vector3::default();
        //const delta: f32 = (1. / 60.) / 3.;
        const delta: f32 = 1.;
        for i in 0..1000 {
            for rps in &device.get_gyro(true)? {
                rotation = Vector3(rps.0 * delta, rps.1 * delta, rps.2 * delta);
            }

            if i % 60 == 0 {
                println!("{:?}", rotation);
            }
        }
    }
    Ok(())
}
