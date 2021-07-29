use std::{
    ops::DerefMut,
    time::{Duration, Instant},
};

use cgmath::{vec3, Deg, Euler, Vector2, Zero};
use enigo::{KeyboardControllable, MouseControllable};
use enum_map::EnumMap;
use hid_gamepad::sys::{JoyKey, KeyStatus, Report};

use crate::{ClickType, calibration::Calibration, config::{settings::Settings, types::GyroSpace}, diff, gyromouse::GyroMouse, joystick::Stick, mapping::{Buttons, ExtAction}, mouse::Mouse, space_mapper::{
        self, LocalSpace, PlayerSpace, SensorFusion, SimpleFusion, SpaceMapper, WorldSpace,
    }};

pub struct Engine {
    left_stick: Box<dyn Stick>,
    right_stick: Box<dyn Stick>,
    buttons: Buttons,
    mouse: Mouse,
    gyro: Gyro,

    last_keys: EnumMap<JoyKey, KeyStatus>,
}

impl Engine {
    pub fn new(settings: Settings, buttons: Buttons, calibration: Calibration) -> Self {
        Engine {
            left_stick: settings.new_left_stick(),
            right_stick: settings.new_right_stick(),
            buttons,
            mouse: Mouse::new(),
            gyro: Gyro::new(settings, calibration),
            last_keys: EnumMap::default(),
        }
    }

    pub fn tick(&mut self, report: Report) -> anyhow::Result<()> {
        let now = Instant::now();

        diff(&mut self.buttons, now, &self.last_keys, &report.keys);
        self.last_keys = report.keys;

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

        self.left_stick.handle(
            report.left_joystick,
            &mut self.buttons,
            &mut self.mouse,
            now,
        );
        self.right_stick.handle(
            report.right_joystick,
            &mut self.buttons,
            &mut self.mouse,
            now,
        );

        self.gyro.handle_frame(report, &mut self.mouse);
        Ok(())
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
    fn handle_frame(&mut self, report: Report, mouse: &mut Mouse) {
        const SMOOTH_RATE: bool = false;
        let mut delta_position = Vector2::zero();
        let dt = 1. / report.frequency as f64;
        for (i, mut frame) in report.motion.into_iter().enumerate() {
            let raw_rot = vec3(
                frame.rotation_speed.x.0,
                frame.rotation_speed.y.0,
                frame.rotation_speed.z.0,
            );
            let rot = raw_rot - self.calibration.get_average();
            frame.rotation_speed = Euler::new(Deg(rot.x), Deg(rot.y), Deg(rot.z));
            let delta = space_mapper::map_input(
                &frame,
                dt,
                self.sensor_fusion.deref_mut(),
                self.space_mapper.deref_mut(),
            ) * 360.
                * 20.;
            let offset = self.gyromouse.process(delta, dt);
            delta_position += offset;
            if self.enabled && !SMOOTH_RATE {
                if i > 0 {
                    std::thread::sleep(Duration::from_secs_f64(dt));
                }
                mouse.mouse_move_relative(offset);
            }
        }
        if self.enabled && SMOOTH_RATE {
            mouse.mouse_move_relative(delta_position);
        }
    }
}
