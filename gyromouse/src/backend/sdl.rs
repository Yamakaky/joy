use std::{
    collections::HashMap,
    thread::sleep,
    time::{Duration, Instant},
};

use anyhow::Result;
use cgmath::{vec2, vec3, Vector3};
use hid_gamepad_types::JoyKey;
use sdl2::{
    self,
    controller::{Axis, Button, GameController},
    event::Event,
    keyboard::Keycode,
    sensor::SensorType,
    GameControllerSubsystem, Sdl,
};

use crate::{
    calibration::Calibration, config::settings::Settings, engine::Engine, mapping::Buttons,
    mouse::Mouse,
};

use super::Backend;

pub struct SDLBackend {
    sdl: Sdl,
    game_controller_system: GameControllerSubsystem,
}

impl SDLBackend {
    pub fn new() -> Result<Self> {
        let sdl = sdl2::init().unwrap();
        let game_controller_system = sdl.game_controller().unwrap();
        Ok(Self {
            sdl,
            game_controller_system,
        })
    }
}

impl Backend for SDLBackend {
    fn list_devices(&mut self) -> anyhow::Result<()> {
        todo!()
    }

    fn run(
        &mut self,
        _opts: crate::opts::Run,
        settings: Settings,
        bindings: Buttons,
        mouse: Mouse,
    ) -> anyhow::Result<()> {
        let mut event_pump = self.sdl.event_pump().unwrap();

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
                        let controller = self.game_controller_system.open(which)?;

                        // Ignore errors, handled later
                        let _ = controller.sensor_set_enabled(SensorType::Accelerometer, true);
                        let _ = controller.sensor_set_enabled(SensorType::Gyroscope, true);

                        let engine = Engine::new(
                            settings.clone(),
                            bindings.clone(),
                            Calibration::empty(),
                            mouse.clone(),
                        );
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

                    engine.apply_motion(gyro.into(), accel.into(), dt);
                }
                engine.apply_actions(now);
            }

            last_tick = now;
            sleep(Duration::from_millis(1));
        }

        Ok(())
    }
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
        Button::Guide => JoyKey::Home,
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
