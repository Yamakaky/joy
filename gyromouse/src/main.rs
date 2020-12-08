mod gyromouse;
mod mapping;

use std::time::{Duration, Instant};

use cgmath::{vec2, Vector2, Zero};
use enigo::MouseControllable;
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
use mapping::{Action, Buttons, JoyKey, Layer};

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
    gyromouse.orientation = Vector2::new(0., 0.);
    gyromouse.apply_tightening = false;
    gyromouse.apply_smoothing = false;
    gyromouse.apply_acceleration = false;
    gyromouse.sensitivity = 32.;
    let mut enigo = enigo::Enigo::new();

    const ACCUMULATE: bool = false;

    let mut mapping = get_mapping();
    let mut last_buttons = ButtonsStatus::default();

    loop {
        let report = device.tick()?;

        diff(&mut mapping, last_buttons, report.buttons);
        mapping.tick(Instant::now());
        last_buttons = report.buttons;

        if let Some(imu) = report.imu {
            let mut delta_position = Vector2::zero();
            for (i, frame) in imu.iter().enumerate() {
                let offset = gyromouse.process(
                    vec2(frame.gyro.z, frame.gyro.y),
                    joycon::IMU::SAMPLE_DURATION,
                );
                delta_position += offset;
                if !ACCUMULATE {
                    if i > 0 {
                        std::thread::sleep(Duration::from_secs_f64(joycon::IMU::SAMPLE_DURATION));
                    }
                    enigo.mouse_move_relative(offset.x as i32, -offset.y as i32);
                }
            }
            if ACCUMULATE {
                enigo.mouse_move_relative(delta_position.x as i32, -delta_position.y as i32);
            }
        }
    }
}

fn get_mapping() -> Buttons {
    let mut mapping = Buttons::new();

    mapping.set_binding(
        JoyKey::S,
        0,
        Layer {
            on_down: Some(Action::KeyPress('a', None)),
            on_up: Some(Action::KeyPress('b', None)),
            ..Default::default()
        },
    );
    mapping.set_binding(
        JoyKey::S,
        1,
        Layer {
            on_down: Some(Action::KeyPress('x', None)),
            on_up: Some(Action::KeyPress('y', None)),
            ..Default::default()
        },
    );
    mapping.set_binding(
        JoyKey::L,
        0,
        Layer {
            on_click: Some(Action::KeyPress('z', None)),
            on_hold_down: Some(Action::Layer(1, true)),
            on_hold_up: Some(Action::Layer(1, false)),
            on_double_click: Some(Action::KeyPress('p', None)),
            ..Default::default()
        },
    );

    mapping
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

fn diff(mapping: &mut Buttons, old: ButtonsStatus, new: ButtonsStatus) {
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
