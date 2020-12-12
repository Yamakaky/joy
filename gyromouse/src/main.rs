mod gyromouse;
mod mapping;
mod parse;

use std::time::{Duration, Instant};

use cgmath::{vec2, Vector2, Zero};
use enigo::{Enigo, Key, KeyboardControllable, MouseControllable};
use gyromouse::GyroMouse;
use joycon::{
    hidapi::{self, HidApi},
    joycon_sys::input::ButtonsStatus,
    joycon_sys::{
        input::BatteryLevel,
        light::{self, PlayerLight},
        NINTENDO_VENDOR_ID,
    },
    JoyCon,
};
use mapping::{Buttons, JoyKey};
use parse::parse_file;

#[derive(Debug, Copy, Clone)]
pub enum ExtAction {
    KeyPress(Key, Option<bool>),
    ToggleGyro(bool),
}

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

    println!("Calibrating...");
    device.enable_imu()?;
    device.load_calibration()?;

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

    let mut gyromouse = GyroMouse::d2();
    let mut enigo = Enigo::new();

    const SMOOTH_RATE: bool = false;
    let mut error_accumulator = Vector2::zero();

    let mut mapping = Buttons::new();
    parse_file("R x\nR,E y\nS a", &mut mapping);
    let mut last_buttons = ButtonsStatus::default();

    let mut gyro_enabled = true;

    loop {
        let report = device.tick()?;

        diff(&mut mapping, last_buttons, report.buttons);
        last_buttons = report.buttons;

        for action in mapping.tick(Instant::now()).drain(..) {
            match action {
                ExtAction::ToggleGyro(gyro) => gyro_enabled = gyro,
                ExtAction::KeyPress(c, None) => enigo.key_click(c),
                ExtAction::KeyPress(c, Some(true)) => enigo.key_down(c),
                ExtAction::KeyPress(c, Some(false)) => enigo.key_up(c),
            }
        }

        if gyro_enabled {
            if let Some(imu) = report.imu {
                let mut delta_position = Vector2::zero();
                for (i, frame) in imu.iter().enumerate() {
                    let offset = gyromouse.process(
                        vec2(frame.gyro.z, frame.gyro.y),
                        joycon::IMU::SAMPLE_DURATION,
                    );
                    delta_position += offset;
                    if !SMOOTH_RATE {
                        if i > 0 {
                            std::thread::sleep(Duration::from_secs_f64(
                                joycon::IMU::SAMPLE_DURATION,
                            ));
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
}

// mouse movement is pixel perfect, so we keep track of the error.
fn mouse_move(enigo: &mut Enigo, offset: Vector2<f64>, error_accumulator: &mut Vector2<f64>) {
    let sum = offset + *error_accumulator;
    let rounded = vec2(sum.x.round(), sum.y.round());
    *error_accumulator = sum - rounded;
    enigo.mouse_move_relative(rounded.x as i32, -rounded.y as i32);
}

macro_rules! diff {
    ($mapping:ident, $now:ident, $old:expr, $new:expr, $side:ident, $member:ident, $key:ident) => {
        if !$old.$side.$member() && $new.$side.$member() {
            $mapping.key_down(JoyKey::$key, $now);
        }
        if $old.$side.$member() && !$new.$side.$member() {
            $mapping.key_up(JoyKey::$key, $now);
        }
    };
}

fn diff(mapping: &mut Buttons<ExtAction>, old: ButtonsStatus, new: ButtonsStatus) {
    let now = Instant::now();
    diff!(mapping, now, old, new, left, up, Up);
    diff!(mapping, now, old, new, left, down, Down);
    diff!(mapping, now, old, new, left, left, Left);
    diff!(mapping, now, old, new, left, right, Right);
    diff!(mapping, now, old, new, left, l, L);
    diff!(mapping, now, old, new, left, zl, ZL);
    diff!(mapping, now, old, new, left, sl, SL);
    diff!(mapping, now, old, new, left, sr, SR);
    diff!(mapping, now, old, new, middle, lstick, L3);
    diff!(mapping, now, old, new, middle, rstick, R3);
    diff!(mapping, now, old, new, middle, minus, Minus);
    diff!(mapping, now, old, new, middle, plus, Plus);
    diff!(mapping, now, old, new, middle, capture, Capture);
    diff!(mapping, now, old, new, middle, home, Home);
    diff!(mapping, now, old, new, right, y, W);
    diff!(mapping, now, old, new, right, x, N);
    diff!(mapping, now, old, new, right, b, S);
    diff!(mapping, now, old, new, right, a, E);
    diff!(mapping, now, old, new, right, r, R);
    diff!(mapping, now, old, new, right, zr, ZR);
    diff!(mapping, now, old, new, right, sl, SL);
    diff!(mapping, now, old, new, right, sr, SR);
}
