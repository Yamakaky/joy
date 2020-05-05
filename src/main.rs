use hidapi::HidApi;

mod hid;

fn main() -> anyhow::Result<()> {
    // Bigger buffer than needed to detect partial reads
    let mut buffer = [0u8; 1 + std::mem::size_of::<joycon_sys::InputReport>()];
    let api = HidApi::new()?;

    for device in api
        .device_list()
        .filter(|x| x.vendor_id() == joycon_sys::NINTENDO_VENDOR_ID)
    {
        let mut device = hid::JoyCon::new(device.open_device(&api)?, device.clone());
        println!("new dev {:?}", device);
        device.enable_imu()?;
        device.set_standard_mode()?;
        device.set_player_light(joycon_sys::PlayerLights::new(
            true, false, false, true, false, false, false, false,
        ))?;
        for _ in 0..3 {
            let report = device.recv(&mut buffer)?;
            println!("{:?}", report);
        }
    }
    Ok(())
}
