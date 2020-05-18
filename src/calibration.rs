use cgmath::*;
use std::collections::VecDeque;

pub const IMU_SAMPLES_PER_SECOND: u32 = 200;

type Entry = Euler<Deg<f32>>;

#[derive(Clone, Debug)]
pub struct Calibration {
    history: VecDeque<Entry>,
    capacity: usize,
}

impl Calibration {
    pub fn with_capacity(capacity: usize) -> Calibration {
        Calibration {
            history: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, entry: Entry) {
        if self.history.len() == self.capacity {
            self.history.pop_back();
        }
        self.history.push_front(entry);
    }

    pub fn reset(&mut self) {
        self.history.clear();
    }

    pub fn get_average(&mut self) -> Entry {
        let zero = Euler::new(Deg(0.), Deg(0.), Deg(0.));
        let len = self.history.len() as f32;
        if len == 0. {
            return zero;
        }
        let sum = self.history.iter().cloned().fold((0., 0., 0.), |acc, val| {
            (
                acc.0 + val.x.0 / len,
                acc.1 + val.y.0 / len,
                acc.2 + val.z.0 / len,
            )
        });
        Euler::new(Deg(sum.0), Deg(sum.1), Deg(sum.2))
    }
}

impl Default for Calibration {
    fn default() -> Self {
        Calibration::with_capacity(3 * IMU_SAMPLES_PER_SECOND as usize)
    }
}
