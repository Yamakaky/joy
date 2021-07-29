use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    thread::sleep,
    time::{Duration, Instant},
};

use anyhow::Context;
use cgmath::{vec2, vec3, Vector3};
use hid_gamepad::sys::JoyKey;
use sdl2::{
    self,
    controller::{Axis, Button, GameController},
    event::Event,
    keyboard::Keycode,
    sensor::SensorType,
};

use crate::{
    calibration::Calibration,
    config::{self, settings::Settings},
    engine::Engine,
    mapping::Buttons,
    opts::Opts,
};

pub fn sdl_main(opts: &Opts) -> anyhow::Result<()> {
    let opts = match opts {
        Opts::Run(r) => r,
        _ => todo!(),
    };
    let sdl = sdl2::init().unwrap();
    let game_controller_system = sdl.game_controller().unwrap();

    let mut event_pump = sdl.event_pump().unwrap();

    let mut controllers = HashMap::new();

    let mut last_tick = Instant::now();

    'running: loop {
        let now = Instant::now();
        let dt = now.duration_since(last_tick);

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::ControllerDeviceAdded { which, .. } => {
                    let controller = game_controller_system.open(which)?;

                    // Ignore errors, handled later
                    let _ = controller.sensor_set_enabled(SensorType::Accelerometer, true);
                    let _ = controller.sensor_set_enabled(SensorType::Gyroscope, true);

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

                    let calibration = Calibration::with_capacity(250 * 60);
                    let engine = Engine::new(settings, bindings, calibration);
                    controllers.insert(which, ControllerState { controller, engine });
                }
                Event::ControllerDeviceRemoved { which, .. } => {
                    controllers.remove(&which);
                }
                Event::ControllerButtonDown {
                    timestamp: _,
                    which,
                    button,
                } => {
                    let controller = controllers.get_mut(&which).unwrap();
                    controller
                        .engine
                        .buttons()
                        .key_down(sdl_to_sys(button), now);
                }
                Event::ControllerButtonUp {
                    timestamp: _,
                    which,
                    button,
                } => {
                    let controller = controllers.get_mut(&which).unwrap();
                    controller.engine.buttons().key_up(sdl_to_sys(button), now);
                }
                _ => {}
            }
        }

        for controller in controllers.values_mut() {
            let c = &mut controller.controller;
            let engine = &mut controller.engine;
            let left = vec2(c.axis(Axis::LeftX), c.axis(Axis::LeftY))
                .cast::<f64>()
                .unwrap()
                / (i16::MAX as f64);
            let right = vec2(c.axis(Axis::RightX), c.axis(Axis::RightY))
                .cast::<f64>()
                .unwrap()
                / (i16::MAX as f64);
            engine.handle_left_stick(left, now);
            engine.handle_right_stick(right, now);
            if c.sensor_enabled(SensorType::Accelerometer)
                && c.sensor_enabled(SensorType::Gyroscope)
            {
                let mut accel = [0.; 3];
                c.sensor_get_data(SensorType::Accelerometer, &mut accel)?;
                let accel = Vector3::from(accel).cast::<f64>().unwrap() / 9.82;
                let mut gyro = [0.; 3];
                c.sensor_get_data(SensorType::Gyroscope, &mut gyro)?;
                let gyro = vec3(gyro[0] as f64, gyro[1] as f64, gyro[2] as f64)
                    / std::f64::consts::PI
                    * 180.;

                engine.apply_motion(gyro, accel, dt);
            }
            engine.apply_actions(now);
        }

        last_tick = now;
        sleep(Duration::from_millis(1));
    }

    Ok(())
}

struct ControllerState {
    controller: GameController,
    engine: Engine,
}

fn sdl_to_sys(button: Button) -> JoyKey {
    match button {
        Button::A => JoyKey::S,
        Button::B => JoyKey::E,
        Button::X => JoyKey::W,
        Button::Y => JoyKey::N,
        Button::Back => JoyKey::Minus,
        Button::Guide => todo!(),
        Button::Start => JoyKey::Plus,
        Button::LeftStick => JoyKey::L3,
        Button::RightStick => JoyKey::R3,
        Button::LeftShoulder => JoyKey::L,
        Button::RightShoulder => JoyKey::R,
        Button::DPadUp => JoyKey::Up,
        Button::DPadDown => JoyKey::Down,
        Button::DPadLeft => JoyKey::Left,
        Button::DPadRight => JoyKey::Right,
    }
}
