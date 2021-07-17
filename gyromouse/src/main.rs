mod calibration;
mod gyromouse;
mod joystick;
mod mapping;
mod mouse;
mod parse;
mod space_mapper;

use std::time::{Duration, Instant};

use cgmath::{vec3, Deg, Euler, Vector2, Zero};
use enigo::{KeyboardControllable, MouseControllable};
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
use joystick::*;
use mapping::{Buttons, ExtAction};
use mouse::Mouse;
use parse::parse_file;

use crate::{calibration::Calibration, space_mapper::*};

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

    let mut gyromouse = GyroMouse::d3();
    let mut mouse = Mouse::new();

    const SMOOTH_RATE: bool = false;

    let mut bindings = Buttons::new();
    //parse_file(
    //    "RLeft = left
    //    RRight = right
    //    RUp = up
    //    RDown = down
    //    W = lmouse
    //    E = rmouse
    //    N = escape
    //    S = none gyro_on\\",
    //    &mut bindings,
    //)?;
    parse_file(
        "LLEFT = q
LRIGHT = d
LUP = z
LDOWN = s
RIGHT = h
UP = SCROLLUP
DOWN = c
ZR = e
ZL = SHIFT
R = LMOUSE
L = RMOUSE
N = a
W = r f
E = none gyro_off
S = SPACE
",
        &mut bindings,
    )?;
    let mut last_buttons = EnumMap::default();

    let mut lstick = ButtonStick::left(false);
    let mut rstick = FlickStick::default();

    let mut gyro_enabled = false;

    let mut calibration = Calibration::with_capacity(250 * 60);
    let mut sensor_fusion = SimpleFusion::new();
    let mut space_mapper = PlayerSpace::default();

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

    loop {
        let mut report = gamepad.recv()?;
        let now = Instant::now();

        diff(&mut bindings, now, &last_buttons, &report.keys);
        last_buttons = report.keys;

        for action in bindings.tick(now).drain(..) {
            match action {
                ExtAction::GyroOn(ClickType::Press) | ExtAction::GyroOff(ClickType::Release) => {
                    gyro_enabled = true
                }
                ExtAction::GyroOn(ClickType::Release) | ExtAction::GyroOff(ClickType::Press) => {
                    gyro_enabled = false
                }
                ExtAction::GyroOn(_) | ExtAction::GyroOff(_) => unimplemented!(),
                ExtAction::KeyPress(c, ClickType::Click) => mouse.enigo().key_click(c),
                ExtAction::KeyPress(c, ClickType::Press) => mouse.enigo().key_down(c),
                ExtAction::KeyPress(c, ClickType::Release) => mouse.enigo().key_up(c),
                ExtAction::KeyPress(_, ClickType::Toggle) => unimplemented!(),
                ExtAction::MousePress(c, ClickType::Click) => mouse.enigo().mouse_click(c),
                ExtAction::MousePress(c, ClickType::Press) => mouse.enigo().mouse_down(c),
                ExtAction::MousePress(c, ClickType::Release) => mouse.enigo().mouse_up(c),
                ExtAction::MousePress(_, ClickType::Toggle) => unimplemented!(),
            }
        }

        lstick.handle(report.left_joystick, &mut bindings, &mut mouse, now);
        rstick.handle(report.right_joystick, &mut bindings, &mut mouse, now);

        let mut delta_position = Vector2::zero();
        let dt = 1. / report.frequency as f64;
        for (i, frame) in report.motion.iter_mut().enumerate() {
            let raw_rot = vec3(
                frame.rotation_speed.x.0,
                frame.rotation_speed.y.0,
                frame.rotation_speed.z.0,
            );
            let rot = raw_rot - calibration.get_average();
            frame.rotation_speed = Euler::new(Deg(rot.x), Deg(rot.y), Deg(rot.z));
            let delta = space_mapper::map_input(frame, dt, &mut sensor_fusion, &mut space_mapper)
                * 360.
                * 20.;
            let offset = gyromouse.process(delta, dt);
            delta_position += offset;
            if gyro_enabled && !SMOOTH_RATE {
                if i > 0 {
                    std::thread::sleep(Duration::from_secs_f64(dt));
                }
                mouse.mouse_move_relative(offset);
            }
        }
        if gyro_enabled && SMOOTH_RATE {
            mouse.mouse_move_relative(delta_position);
        }
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
