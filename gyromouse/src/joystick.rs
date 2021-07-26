#![allow(dead_code)]

use std::time::{Duration, Instant};

use cgmath::{vec2, AbsDiffEq, Angle, Deg, InnerSpace, Rad, Vector2};

use crate::{
    config::{settings::FlickStickSettings, types::RingMode},
    mapping::{Buttons, VirtualKey},
    mouse::Mouse,
};

pub trait Stick {
    fn handle(
        &mut self,
        stick: Vector2<f64>,
        bindings: &mut Buttons,
        mouse: &mut Mouse,
        now: Instant,
    );
}

pub struct CameraStick {
    deadzone: f64,
    fullzone: f64,
    sens_pps: f64,
    exp: f64,
    acceleration: f64,
    max_speed: f64,
    current_speed: f64,
}

impl Default for CameraStick {
    fn default() -> Self {
        CameraStick {
            deadzone: 0.15,
            fullzone: 0.9,
            sens_pps: 1000.,
            exp: 2.,
            acceleration: 0.,
            max_speed: 10.,
            current_speed: 0.,
        }
    }
}

impl Stick for CameraStick {
    fn handle(
        &mut self,
        stick: Vector2<f64>,
        _bindings: &mut Buttons,
        mouse: &mut Mouse,
        _now: Instant,
    ) {
        let amp = stick.magnitude();
        let amp_zones = (amp - self.deadzone) / (self.fullzone - self.deadzone);
        if amp_zones >= 1. {
            self.current_speed = (self.current_speed + self.acceleration / 66.).min(self.max_speed);
        } else {
            self.current_speed = 0.;
        }
        let amp_clamped = amp_zones.max(0.).min(1.);
        let amp_exp = amp_clamped.powf(self.exp);
        mouse.mouse_move_relative(
            self.sens_pps / 66. * (1. + self.current_speed) * stick.normalize_to(amp_exp),
        );
    }
}

#[derive(Debug)]
enum FlickStickState {
    Center,
    Flicking {
        flick_start: Instant,
        last: Deg<f64>,
        target: Deg<f64>,
    },
    Rotating {
        old_rotation: Deg<f64>,
    },
}

#[derive(Debug)]
pub struct FlickStick {
    flick_time: Duration,
    threshold: f64,
    state: FlickStickState,
    do_rotate: bool,
    do_flick: bool,
}

impl Default for FlickStick {
    fn default() -> Self {
        FlickStick {
            flick_time: Duration::from_millis(200),
            threshold: 0.6,
            state: FlickStickState::Center,
            do_rotate: true,
            do_flick: true,
        }
    }
}

impl FlickStick {
    pub fn new(settings: &FlickStickSettings, threshold: f64, flick: bool, rotate: bool) -> Self {
        Self {
            flick_time: settings.flick_time,
            threshold,
            state: FlickStickState::Center,
            do_rotate: rotate,
            do_flick: flick,
        }
    }
}

impl Stick for FlickStick {
    fn handle(
        &mut self,
        stick: Vector2<f64>,
        _bindings: &mut Buttons,
        mouse: &mut Mouse,
        now: Instant,
    ) {
        let offset = match self.state {
            FlickStickState::Center | FlickStickState::Rotating { .. }
                if stick.magnitude() < self.threshold =>
            {
                self.state = FlickStickState::Center;
                None
            }
            FlickStickState::Center => {
                let target = stick.angle(Vector2::unit_y()).into();
                self.state = if self.do_flick {
                    FlickStickState::Flicking {
                        flick_start: now,
                        last: Deg(0.),
                        target,
                    }
                } else {
                    FlickStickState::Rotating {
                        old_rotation: target,
                    }
                };
                None
            }
            FlickStickState::Flicking {
                flick_start,
                ref mut last,
                target,
            } => {
                let elapsed = now.duration_since(flick_start).as_secs_f64();
                let max = self.flick_time.as_secs_f64() * target.0.abs() / 180.;
                let dt_factor = elapsed / max;
                let current_angle = target * dt_factor.min(1.);
                let delta = current_angle - *last;
                if dt_factor > 1. {
                    self.state = FlickStickState::Rotating {
                        old_rotation: current_angle,
                    };
                } else {
                    *last = current_angle;
                }
                Some(delta.normalize_signed())
            }
            FlickStickState::Rotating {
                ref mut old_rotation,
            } => {
                if self.do_rotate {
                    let angle = stick.angle(Vector2::unit_y()).into();
                    let delta = angle - *old_rotation;
                    *old_rotation = angle;
                    Some(delta.normalize_signed())
                } else {
                    None
                }
            }
        };
        if let Some(offset) = offset {
            mouse.mouse_move_relative(vec2(offset.0, 0.) * 25.);
        }
    }
}

pub struct ButtonStick {
    deadzone: f64,
    fullzone: f64,
    left: bool,
    angle: Deg<f64>,
    ring_mode: RingMode,
}

impl ButtonStick {
    pub fn left(ring_mode: RingMode) -> Self {
        Self {
            deadzone: 0.15,
            fullzone: 0.9,
            left: true,
            angle: Deg(30.),
            ring_mode,
        }
    }

    pub fn right(ring_mode: RingMode) -> Self {
        Self {
            deadzone: 0.15,
            fullzone: 0.9,
            left: false,
            angle: Deg(30.),
            ring_mode,
        }
    }
}

impl Stick for ButtonStick {
    fn handle(
        &mut self,
        stick: Vector2<f64>,
        bindings: &mut Buttons,
        _mouse: &mut Mouse,
        _now: Instant,
    ) {
        let amp = stick.magnitude();
        let amp_zones = (amp - self.deadzone) / (self.fullzone - self.deadzone);
        let amp_clamped = amp_zones.max(0.).min(1.);
        let stick = stick.normalize_to(amp_clamped);
        let now = std::time::Instant::now();

        let epsilon = Rad::from(Deg(90.) - self.angle).0;

        let angle_r = stick.angle(Vector2::unit_x());
        let angle_l = stick.angle(-Vector2::unit_x());
        let angle_u = stick.angle(Vector2::unit_y());
        let angle_d = stick.angle(-Vector2::unit_y());

        if amp_clamped > 0. {
            bindings.key(
                if self.left {
                    VirtualKey::LRing
                } else {
                    VirtualKey::RRing
                },
                match self.ring_mode {
                    RingMode::Inner => amp_clamped < 1.,
                    RingMode::Outer => amp_clamped >= 1.,
                },
                now,
            );
            bindings.key(
                if self.left {
                    VirtualKey::LRight
                } else {
                    VirtualKey::RRight
                },
                angle_r.abs_diff_eq(&Rad(0.), epsilon),
                now,
            );
            bindings.key(
                if self.left {
                    VirtualKey::LLeft
                } else {
                    VirtualKey::RLeft
                },
                angle_l.abs_diff_eq(&Rad(0.), epsilon),
                now,
            );
            bindings.key(
                if self.left {
                    VirtualKey::LUp
                } else {
                    VirtualKey::RUp
                },
                angle_u.abs_diff_eq(&Rad(0.), epsilon),
                now,
            );
            bindings.key(
                if self.left {
                    VirtualKey::LDown
                } else {
                    VirtualKey::RDown
                },
                angle_d.abs_diff_eq(&Rad(0.), epsilon),
                now,
            );
        } else if self.left {
            bindings.key_up(VirtualKey::LLeft, now);
            bindings.key_up(VirtualKey::LRight, now);
            bindings.key_up(VirtualKey::LUp, now);
            bindings.key_up(VirtualKey::LDown, now);
        } else {
            bindings.key_up(VirtualKey::RLeft, now);
            bindings.key_up(VirtualKey::RRight, now);
            bindings.key_up(VirtualKey::RUp, now);
            bindings.key_up(VirtualKey::RDown, now);
        }
    }
}
