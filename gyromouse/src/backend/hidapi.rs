use std::{fs::File, io::Read};

use crate::{
    calibration::Calibration,
    config::{self, settings::Settings},
    engine::Engine,
    mapping::Buttons,
    opts::{Opts, Run},
};

use anyhow::Context;
use cgmath::vec3;
use hid_gamepad::sys::GamepadDevice;
use joycon::{
    hidapi::HidApi,
    joycon_sys::{
        input::BatteryLevel,
        light::{self, PlayerLight},
    },
    JoyCon,
};
use nom::{error::convert_error, Err};

pub fn hidapi_main(opts: &Opts) -> anyhow::Result<()> {
    let mut api = HidApi::new()?;
    match opts {
        Opts::Validate(run) => {
            let mut settings = Settings::default();
            let mut bindings = Buttons::new();
            let mut content_file = File::open(&run.mapping_file)
                .with_context(|| format!("opening config file {}", run.mapping_file))?;
            let content = {
                let mut buf = String::new();
                content_file.read_to_string(&mut buf)?;
                buf
            };
            match config::parse::parse_file(&content, &mut settings, &mut bindings) {
                Ok(_) => {}
                Err(Err::Error(e)) | Err(Err::Failure(e)) => {
                    println!("{:?}", convert_error(content.as_str(), e))
                }
                Err(_) => unimplemented!(),
            }
        }
        Opts::List => {
            println!("Listing gamepads:");
            for device_info in api.device_list() {
                if hid_gamepad::open_gamepad(&api, device_info)?.is_some() {
                    println!("Found one");
                }
            }
        }
        Opts::Run(run) => loop {
            for device_info in api.device_list() {
                if let Some(mut gamepad) = hid_gamepad::open_gamepad(&api, device_info)? {
                    return hid_main(gamepad.as_mut(), &run);
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
            api.refresh_devices()?;
        },
        Opts::FlickCalibrate => {
            todo!()
        }
    }
    Ok(())
}

fn hid_main(gamepad: &mut dyn GamepadDevice, opts: &Run) -> anyhow::Result<()> {
    if let Some(joycon) = gamepad.as_any().downcast_mut::<JoyCon>() {
        dbg!(joycon.set_home_light(light::HomeLight::new(
            0x8,
            0x2,
            0x0,
            &[(0xf, 0xf, 0), (0x2, 0xf, 0)],
        ))?);

        let battery_level = joycon.tick()?.info.battery_level();

        joycon.set_player_light(light::PlayerLights::new(
            (battery_level >= BatteryLevel::Full).into(),
            (battery_level >= BatteryLevel::Medium).into(),
            (battery_level >= BatteryLevel::Low).into(),
            if battery_level >= BatteryLevel::Low {
                PlayerLight::On
            } else {
                PlayerLight::Blinking
            },
        ))?;
    }

    let mut settings = Settings::default();
    let mut bindings = Buttons::new();
    let mut content_file = File::open(&opts.mapping_file)
        .with_context(|| format!("opening config file {}", opts.mapping_file))?;
    let content = {
        let mut buf = String::new();
        content_file.read_to_string(&mut buf)?;
        buf
    };
    config::parse::parse_file(&content, &mut settings, &mut bindings).unwrap();

    let mut calibration = Calibration::with_capacity(250 * 60);

    println!("calibrating");
    for _ in 0..1000 {
        let report = gamepad.recv()?;
        for frame in report.motion.iter() {
            let raw_rot = vec3(
                frame.rotation_speed.x.0,
                frame.rotation_speed.y.0,
                frame.rotation_speed.z.0,
            );
            calibration.push(raw_rot);
        }
    }
    println!("calibrating done");
    let mut engine = Engine::new(settings, bindings, calibration);

    loop {
        let report = gamepad.recv()?;
        engine.tick(report)?;
    }
}
