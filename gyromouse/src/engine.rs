use std::{
    ops::DerefMut,
    time::{Duration, Instant},
};

use cgmath::{Vector2, Zero};
use enigo::{KeyboardControllable, MouseControllable};
use enum_map::EnumMap;
use hid_gamepad_types::{Acceleration, JoyKey, KeyStatus, Motion, Report, RotationSpeed};

use crate::{
    calibration::Calibration,
    config::{settings::Settings, types::GyroSpace},
    gyromouse::GyroMouse,
    joystick::Stick,
    mapping::{Buttons, ExtAction},
    mouse::Mouse,
    space_mapper::{
        self, LocalSpace, PlayerSpace, SensorFusion, SimpleFusion, SpaceMapper, WorldSpace,
    },
    ClickType,
};

pub struct Engine {
    left_stick: Box<dyn Stick>,
    right_stick: Box<dyn Stick>,
    buttons: Buttons,
    mouse: Mouse,
    gyro: Gyro,

    last_keys: EnumMap<JoyKey, KeyStatus>,
}

impl Engine {
    pub fn new(
        settings: Settings,
        buttons: Buttons,
        calibration: Calibration,
        mouse: Mouse,
    ) -> Self {
        Engine {
            left_stick: settings.new_left_stick(),
            right_stick: settings.new_right_stick(),
            buttons,
            mouse,
            gyro: Gyro::new(settings, calibration),
            last_keys: EnumMap::default(),
        }
    }

    pub fn tick(&mut self, report: Report) -> anyhow::Result<()> {
        let now = Instant::now();

        diff(&mut self.buttons, now, &self.last_keys, &report.keys);
        self.last_keys = report.keys;

        self.handle_left_stick(report.left_joystick, now);
        self.handle_right_stick(report.right_joystick, now);

        self.apply_actions(now);

        // dt of the entire report time
        let dt = Duration::from_secs_f64(1. / report.frequency as f64 * report.motion.len() as f64);
        self.gyro.handle_frame(&report.motion, &mut self.mouse, dt);
        Ok(())
    }

    pub fn buttons(&mut self) -> &mut Buttons {
        &mut self.buttons
    }

    pub fn handle_left_stick(&mut self, stick: Vector2<f64>, now: Instant) {
        self.left_stick
            .handle(stick, &mut self.buttons, &mut self.mouse, now);
    }

    pub fn handle_right_stick(&mut self, stick: Vector2<f64>, now: Instant) {
        self.right_stick
            .handle(stick, &mut self.buttons, &mut self.mouse, now);
    }

    pub fn apply_actions(&mut self, now: Instant) {
        for action in self.buttons.tick(now).drain(..) {
            match action {
                ExtAction::GyroOn(ClickType::Press) | ExtAction::GyroOff(ClickType::Release) => {
                    self.gyro.enabled = true
                }
                ExtAction::GyroOn(ClickType::Release) | ExtAction::GyroOff(ClickType::Press) => {
                    self.gyro.enabled = false
                }
                ExtAction::GyroOn(_) | ExtAction::GyroOff(_) => unimplemented!(),
                ExtAction::KeyPress(c, ClickType::Click) => self.mouse.enigo().key_click(c),
                ExtAction::KeyPress(c, ClickType::Press) => self.mouse.enigo().key_down(c),
                ExtAction::KeyPress(c, ClickType::Release) => self.mouse.enigo().key_up(c),
                ExtAction::KeyPress(_, ClickType::Toggle) => unimplemented!(),
                ExtAction::MousePress(c, ClickType::Click) => self.mouse.enigo().mouse_click(c),
                ExtAction::MousePress(c, ClickType::Press) => self.mouse.enigo().mouse_down(c),
                ExtAction::MousePress(c, ClickType::Release) => self.mouse.enigo().mouse_up(c),
                ExtAction::MousePress(_, ClickType::Toggle) => unimplemented!(),
            }
        }
    }

    pub fn apply_motion(
        &mut self,
        rotation_speed: RotationSpeed,
        acceleration: Acceleration,
        dt: Duration,
    ) {
        self.gyro.handle_frame(
            &[Motion {
                rotation_speed,
                acceleration,
            }],
            &mut self.mouse,
            dt,
        )
    }
}

pub struct Gyro {
    enabled: bool,
    calibration: Calibration,
    sensor_fusion: Box<dyn SensorFusion>,
    space_mapper: Box<dyn SpaceMapper>,
    gyromouse: GyroMouse,
}

impl Gyro {
    pub fn new(settings: Settings, calibration: Calibration) -> Gyro {
        Gyro {
            enabled: true,
            calibration,
            sensor_fusion: Box::new(SimpleFusion::new()),
            space_mapper: match settings.gyro.space {
                GyroSpace::Local => Box::new(LocalSpace::default()),
                GyroSpace::WorldTurn => Box::new(WorldSpace::default()),
                GyroSpace::WorldLean => todo!(),
                GyroSpace::PlayerTurn => Box::new(PlayerSpace::default()),
                GyroSpace::PlayerLean => todo!(),
            },
            gyromouse: GyroMouse::from(settings.gyro),
        }
    }
    pub fn handle_frame(&mut self, motions: &[Motion], mouse: &mut Mouse, dt: Duration) {
        const SMOOTH_RATE: bool = false;
        let mut delta_position = Vector2::zero();
        let dt = dt / motions.len() as u32;
        for (i, frame) in motions.iter().cloned().enumerate() {
            let frame = self.calibration.calibrate(frame);
            let delta = space_mapper::map_input(
                &frame,
                dt,
                self.sensor_fusion.deref_mut(),
                self.space_mapper.deref_mut(),
            );
            let offset = self.gyromouse.process(delta, dt);
            delta_position += offset;
            if self.enabled && !SMOOTH_RATE {
                if i > 0 {
                    std::thread::sleep(dt);
                }
                mouse.mouse_move_relative(offset);
            }
        }
        if self.enabled && SMOOTH_RATE {
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
