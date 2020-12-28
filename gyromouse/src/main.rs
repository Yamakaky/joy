mod gyromouse;
mod joystick;
mod mapping;
mod parse;

use std::time::{Duration, Instant};

use cgmath::{vec2, InnerSpace, Vector2, Zero};
use enigo::{Enigo, Key, KeyboardControllable, MouseButton, MouseControllable};
use enum_map::EnumMap;
use gyromouse::GyroMouse;
use hid_gamepad::sys::{GamepadDevice, JoyKey, KeyStatus};
use joycon::{
    hidapi::HidApi,
    joycon_sys::{
        input::BatteryLevel,
        light::{self, PlayerLight},
    },
    JoyCon,
};
use joystick::{ButtonStick, CameraStick};
use mapping::Buttons;
use parse::parse_file;

#[derive(Debug, Copy, Clone)]
pub enum ExtAction {
    KeyPress(Key, ClickType),
    MousePress(MouseButton, ClickType),
    ToggleGyro(ClickType),
}

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
            ClickType::Press => true,
            ClickType::Release => false,
            ClickType::Click => unimplemented!(),
            ClickType::Toggle => !val,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let mut api = HidApi::new()?;
    loop {
        api.refresh_devices()?;
        for device_info in api.device_list() {
            if let Some(mut gamepad) = hid_gamepad::open_gamepad(&api, device_info)? {
                return hid_main(gamepad.as_mut());
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn hid_main(gamepad: &mut dyn GamepadDevice) -> anyhow::Result<()> {
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

    let mut gyromouse = GyroMouse::d2();
    let mut enigo = Enigo::new();

    const SMOOTH_RATE: bool = false;
    let mut error_accumulator = Vector2::zero();

    let mut mapping = Buttons::new();
    parse_file(
        "LLeft = a\nLRight = d\nLUp = w\nLDown  =s\nR =x\nR,E= y\nS =a",
        &mut mapping,
    )?;
    let mut last_buttons = EnumMap::new();

    let mut lstick = ButtonStick::left(0.4);
    let mut rstick = CameraStick::default();

    let mut gyro_enabled = true;

    loop {
        let report = gamepad.recv()?;

        diff(&mut mapping, &last_buttons, &report.keys);
        last_buttons = report.keys;

        for action in mapping.tick(Instant::now()).drain(..) {
            match action {
                ExtAction::ToggleGyro(gyro) => gyro_enabled = gyro.apply(gyro_enabled),
                ExtAction::KeyPress(c, ClickType::Click) => enigo.key_click(c),
                ExtAction::KeyPress(c, ClickType::Press) => enigo.key_down(c),
                ExtAction::KeyPress(c, ClickType::Release) => enigo.key_up(c),
                ExtAction::KeyPress(_, ClickType::Toggle) => unimplemented!(),
                ExtAction::MousePress(c, ClickType::Click) => enigo.mouse_click(c),
                ExtAction::MousePress(c, ClickType::Press) => enigo.mouse_down(c),
                ExtAction::MousePress(c, ClickType::Release) => enigo.mouse_up(c),
                ExtAction::MousePress(_, ClickType::Toggle) => unimplemented!(),
            }
        }

        lstick.handle(report.left_joystick, &mut mapping);
        let offset = rstick.handle(report.right_joystick);
        if offset.magnitude() != 0. {
            dbg!(offset);
            mouse_move(&mut enigo, offset, &mut error_accumulator);
        }

        if gyro_enabled {
            let mut delta_position = Vector2::zero();
            let dt = 1. / report.frequency as f64;
            for (i, frame) in report.motion.iter().enumerate() {
                let offset =
                    gyromouse.process(vec2(frame.rotation_speed.y.0, frame.rotation_speed.x.0), dt);
                delta_position += offset;
                if !SMOOTH_RATE {
                    if i > 0 {
                        std::thread::sleep(Duration::from_secs_f64(dt));
                    }
                    mouse_move(&mut enigo, offset, &mut error_accumulator);
                }
            }
            if SMOOTH_RATE {
                mouse_move(&mut enigo, delta_position, &mut error_accumulator);
            }
        }
    }
}

// mouse movement is pixel perfect, so we keep track of the error.
fn mouse_move(enigo: &mut Enigo, offset: Vector2<f64>, error_accumulator: &mut Vector2<f64>) {
    let sum = offset + *error_accumulator;
    let rounded = vec2(sum.x.round(), sum.y.round());
    *error_accumulator = sum - rounded;
    if rounded != Vector2::zero() {
        enigo.mouse_move_relative(rounded.x as i32, -rounded.y as i32);
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
    mapping: &mut Buttons<ExtAction>,
    old: &EnumMap<JoyKey, KeyStatus>,
    new: &EnumMap<JoyKey, KeyStatus>,
) {
    use JoyKey::*;

    let now = Instant::now();
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
