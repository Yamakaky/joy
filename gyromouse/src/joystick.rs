use cgmath::{AbsDiffEq, Deg, InnerSpace, Rad, Vector2};

use crate::{
    mapping::{Buttons, JoyKey},
    ExtAction,
};

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
            deadzone: 0.1,
            fullzone: 0.9,
            sens_pps: 1000.,
            exp: 2.,
            acceleration: 0.,
            max_speed: 10.,
            current_speed: 0.,
        }
    }
}

impl CameraStick {
    pub fn handle(&mut self, stick: Vector2<f64>) -> Vector2<f64> {
        let amp = stick.magnitude();
        let amp_zones = (amp - self.deadzone) / (self.fullzone - self.deadzone);
        if amp_zones >= 1. {
            self.current_speed = (self.current_speed + self.acceleration / 66.).min(self.max_speed);
        } else {
            self.current_speed = 0.;
        }
        let amp_clamped = amp_zones.max(0.).min(1.);
        let amp_exp = amp_clamped.powf(self.exp);
        self.sens_pps / 66. * (1. + self.current_speed) * stick.normalize_to(amp_exp)
    }
}

pub struct ButtonStick {
    deadzone: f64,
    left: bool,
    angle: Deg<f64>,
}

impl ButtonStick {
    pub fn left(deadzone: f64) -> Self {
        Self {
            deadzone,
            left: true,
            angle: Deg(30.),
        }
    }

    pub fn right(deadzone: f64) -> Self {
        Self {
            deadzone,
            left: false,
            angle: Deg(30.),
        }
    }

    pub fn handle(&mut self, stick: Vector2<f64>, bindings: &mut Buttons<ExtAction>) {
        let amp = stick.magnitude();
        let amp_zones = (amp - self.deadzone) / (1. - self.deadzone);
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
                    JoyKey::LRight
                } else {
                    JoyKey::RRight
                },
                angle_r.abs_diff_eq(&Rad(0.), epsilon),
                now,
            );
            bindings.key(
                if self.left {
                    JoyKey::LLeft
                } else {
                    JoyKey::RLeft
                },
                angle_l.abs_diff_eq(&Rad(0.), epsilon),
                now,
            );
            bindings.key(
                if self.left { JoyKey::LUp } else { JoyKey::RUp },
                angle_u.abs_diff_eq(&Rad(0.), epsilon),
                now,
            );
            bindings.key(
                if self.left {
                    JoyKey::LDown
                } else {
                    JoyKey::RDown
                },
                angle_d.abs_diff_eq(&Rad(0.), epsilon),
                now,
            );
        } else if self.left {
            bindings.key_up(JoyKey::LLeft, now);
            bindings.key_up(JoyKey::LRight, now);
            bindings.key_up(JoyKey::LUp, now);
            bindings.key_up(JoyKey::LDown, now);
        } else {
            bindings.key_up(JoyKey::RLeft, now);
            bindings.key_up(JoyKey::RRight, now);
            bindings.key_up(JoyKey::RUp, now);
            bindings.key_up(JoyKey::RDown, now);
        }
    }
}
