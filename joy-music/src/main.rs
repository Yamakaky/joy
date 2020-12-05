use hidapi::HidApi;
use joycon::{
    joycon_sys::{
        input::BatteryLevel,
        light::{self, PlayerLight},
        output::RumbleData,
        output::RumbleSide,
        NINTENDO_VENDOR_ID,
    },
    JoyCon,
};

fn main() -> anyhow::Result<()> {
    let mut api = HidApi::new()?;
    loop {
        api.refresh_devices()?;
        if let Some(device_info) = api
            .device_list()
            .find(|x| x.vendor_id() == NINTENDO_VENDOR_ID)
        {
            let device = device_info.open_device(&api)?;
            match hid_main(device, device_info) {
                Ok(()) => std::thread::sleep(std::time::Duration::from_secs(2)),
                Err(e) => println!("Joycon error: {}", e),
            }
        } else {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}

fn hid_main(device: hidapi::HidDevice, device_info: &hidapi::DeviceInfo) -> anyhow::Result<()> {
    let mut device = JoyCon::new(device, device_info.clone())?;
    println!("new dev: {:?}", device.get_dev_info()?);

    dbg!(device.set_home_light(light::HomeLight::new(
        0x8,
        0x2,
        0x0,
        &[(0xf, 0xf, 0), (0x2, 0xf, 0)],
    ))?);

    let battery_level = device.tick()?.info.battery_level();

    device.set_player_light(light::PlayerLights::new(
        (battery_level >= BatteryLevel::Full).into(),
        (battery_level >= BatteryLevel::Medium).into(),
        (battery_level >= BatteryLevel::Low).into(),
        if battery_level >= BatteryLevel::Low {
            PlayerLight::On
        } else {
            PlayerLight::Blinking
        },
    ))?;

    println!("Running...");
    let mut freq = 261.63;
    let step = [2, 2, 1, 2, 2, 2, 1];
    let mut i = 0;
    while freq < 1050. {
        dbg!(freq);

        let rumble = RumbleSide::from_freq(freq, 0.4, 400., 0.);
        device.set_rumble(RumbleData {
            left: rumble,
            right: rumble,
        })?;

        freq *= 1.0594630943f32.powi(step[i]);
        i = (i + 1) % step.len();

        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    dbg!(device.set_home_light(light::HomeLight::new(0x8, 0x4, 0x0, &[]))?);

    Ok(())
}
