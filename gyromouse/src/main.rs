mod calibration;
mod config;
mod engine;
mod gyromouse;
mod joystick;
mod mapping;
mod mouse;
mod opts;
mod space_mapper;

use std::{fs::File, io::Read, time::Instant};

use anyhow::Context;
use cgmath::vec3;
use clap::Clap;
use enum_map::EnumMap;
use hid_gamepad::sys::{GamepadDevice, JoyKey, KeyStatus};
use joycon::{
    hidapi::HidApi,
    joycon_sys::{
        input::BatteryLevel,
        light::{self, PlayerLight},
    },
    JoyCon,
};
use mapping::Buttons;
use opts::{Opts, Run};

use crate::{calibration::Calibration, engine::Engine};

#[derive(Debug, Copy, Clone)]
pub enum ClickType {
    Press,
    Release,
    Click,
    Toggle,
}

impl ClickType {
    pub fn apply(self, val: bool) -> bool {
        match self {
            ClickType::Press => false,
            ClickType::Release => true,
            ClickType::Click => unimplemented!(),
            ClickType::Toggle => !val,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let mut api = HidApi::new()?;
    let opts = Opts::parse();
    match opts {
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

    let mut bindings = Buttons::new();
    let mut content_file = File::open(&opts.mapping_file)
        .with_context(|| format!("opening config file {}", opts.mapping_file))?;
    let content = {
        let mut buf = String::new();
        content_file.read_to_string(&mut buf)?;
        buf
    };
    config::parse::parse_file(&content, &mut bindings).unwrap();

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
    let mut engine = Engine::new(bindings, calibration);

    loop {
        let report = gamepad.recv()?;
        engine.tick(report)?;
    }
}

macro_rules! diff {
    ($mapping:ident, $now:ident, $old:expr, $new:expr, $key:ident) => {
        match ($old[$key], $new[$key]) {
            (KeyStatus::Released, KeyStatus::Pressed) => $mapping.key_down($key, $now),
            (KeyStatus::Pressed, KeyStatus::Released) => $mapping.key_up($key, $now),
            _ => (),
        }
    };
}

fn diff(
    mapping: &mut Buttons,
    now: Instant,
    old: &EnumMap<JoyKey, KeyStatus>,
    new: &EnumMap<JoyKey, KeyStatus>,
) {
    use JoyKey::*;

    diff!(mapping, now, old, new, Up);
    diff!(mapping, now, old, new, Down);
    diff!(mapping, now, old, new, Left);
    diff!(mapping, now, old, new, Right);
    diff!(mapping, now, old, new, L);
    diff!(mapping, now, old, new, ZL);
    diff!(mapping, now, old, new, SL);
    diff!(mapping, now, old, new, SR);
    diff!(mapping, now, old, new, L3);
    diff!(mapping, now, old, new, R3);
    diff!(mapping, now, old, new, Minus);
    diff!(mapping, now, old, new, Plus);
    diff!(mapping, now, old, new, Capture);
    diff!(mapping, now, old, new, Home);
    diff!(mapping, now, old, new, W);
    diff!(mapping, now, old, new, N);
    diff!(mapping, now, old, new, S);
    diff!(mapping, now, old, new, E);
    diff!(mapping, now, old, new, R);
    diff!(mapping, now, old, new, ZR);
    diff!(mapping, now, old, new, SL);
    diff!(mapping, now, old, new, SR);
}
