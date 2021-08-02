use cgmath::{vec2, Vector2, Zero};
use enigo::{Enigo, MouseControllable};

#[derive(Debug)]
pub struct Mouse {
    enigo: Enigo,
    error_accumulator: Vector2<f64>,
    calibration: f64,
}

impl Mouse {
    pub fn new() -> Self {
        Mouse {
            enigo: Enigo::new(),
            error_accumulator: Vector2::zero(),
            calibration: 1.,
        }
    }

    pub fn clone(&self) -> Self {
        Mouse {
            calibration: self.calibration,
            ..Self::new()
        }
    }

    // mouse movement is pixel perfect, so we keep track of the error.
    pub fn mouse_move_relative(&mut self, mut offset: Vector2<f64>) {
        offset *= self.calibration;
        let sum = offset + self.error_accumulator;
        let rounded = vec2(sum.x.round(), sum.y.round());
        self.error_accumulator = sum - rounded;
        if rounded != Vector2::zero() {
            self.enigo
                .mouse_move_relative(rounded.x as i32, -rounded.y as i32);
        }
    }

    pub fn enigo(&mut self) -> &mut Enigo {
        &mut self.enigo
    }

    pub fn set_calibration(&mut self, calibration: f64) {
        self.calibration = calibration;
    }
}
