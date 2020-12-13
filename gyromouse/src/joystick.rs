use cgmath::{InnerSpace, Vector2};

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
