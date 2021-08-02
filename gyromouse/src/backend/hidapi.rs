use std::time::{Duration, Instant};

use crate::{
    calibration::BetterCalibration, config::settings::Settings, engine::Engine, mapping::Buttons,
    mouse::Mouse, opts::Run,
};

use anyhow::{bail, Result};
use hid_gamepad::sys::GamepadDevice;
use joycon::{
    hidapi::HidApi,
    joycon_sys::{
        input::BatteryLevel,
        light::{self, PlayerLight},
    },
    JoyCon,
};

use super::Backend;

pub struct HidapiBackend {
    api: HidApi,
}

impl HidapiBackend {
    pub fn new() -> Result<Self> {
        Ok(Self {
            api: HidApi::new()?,
        })
    }
}

impl Backend for HidapiBackend {
    fn list_devices(&mut self) -> Result<()> {
        println!("Listing gamepads:");
        for device_info in self.api.device_list() {
            if hid_gamepad::open_gamepad(&self.api, device_info)?.is_some() {
                println!("Found one");
                return Ok(());
            }
        }
        bail!("No gamepad found");
    }

    fn run(
        &mut self,
        _opts: Run,
        settings: Settings,
        bindings: Buttons,
        mouse: Mouse,
    ) -> Result<()> {
        loop {
            for device_info in self.api.device_list() {
                if let Some(mut gamepad) = hid_gamepad::open_gamepad(&self.api, device_info)? {
                    return hid_main(gamepad.as_mut(), settings, bindings, mouse);
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
            self.api.refresh_devices()?;
        }
    }
}

fn hid_main(
    gamepad: &mut dyn GamepadDevice,
    settings: Settings,
    bindings: Buttons,
    mouse: Mouse,
) -> Result<()> {
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

    let mut calibrator = BetterCalibration::default();

    println!("calibrating");
    loop {
        let report = gamepad.recv()?;
        if calibrator.push(report.motion[0], Instant::now(), Duration::from_secs(1)) {
            break;
        }
    }
    println!("calibrating done");
    let mut engine = Engine::new(settings, bindings, calibrator.finish(), mouse);

    loop {
        let report = gamepad.recv()?;
        engine.tick(report)?;
    }
}
