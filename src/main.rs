use hidapi::HidApi;

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

        device.set_nfc_ir_mode()?;
        device.enable_mcu()?;
        device.disable_mcu()?;

        device.enable_imu()?;
        device.set_standard_mode()?;
        device.set_player_light(joycon_sys::output::PlayerLights::new(
            true, false, false, true, false, false, false, false,
        ))?;

        for _ in 0..3 {
            let report = device.recv()?;
            println!("{:?}", report);
        }
    }
    Ok(())
}
