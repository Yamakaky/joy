use cgmath::{vec2, Vector2, Zero};
use enigo::{Enigo, MouseControllable};

#[derive(Debug)]
pub struct Mouse {
    enigo: Enigo,
    error_accumulator: Vector2<f64>,
    calibration: f64,
    game_sens: f64,
    counter_os_speed: bool,
}

impl Mouse {
    pub fn new() -> Self {
        Mouse {
            enigo: Enigo::new(),
            error_accumulator: Vector2::zero(),
            calibration: 1.,
            game_sens: 1.,
            counter_os_speed: false,
        }
    }

    pub fn clone(&self) -> Self {
        Mouse {
            calibration: self.calibration,
            game_sens: self.game_sens,
            counter_os_speed: self.counter_os_speed,
            ..Self::new()
        }
    }

    // mouse movement is pixel perfect, so we keep track of the error.
    pub fn mouse_move_relative(&mut self, mut offset: Vector2<f64>) {
        offset *= self.calibration * self.game_sens;
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

    pub fn set_game_sens(&mut self, sens: f64) {
        self.game_sens = sens;
    }

    pub fn set_counter_os_speed(&mut self, counter: bool) {
        println!("Warning: counter os speed not supported");
        self.counter_os_speed = counter;
    }
}
