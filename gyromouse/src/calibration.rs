use cgmath::*;
use std::collections::VecDeque;

type Entry = Vector3<f64>;

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
        let zero = Vector3::new(0., 0., 0.);
        let len = self.history.len() as f64;
        if len == 0. {
            return zero;
        }
        self.history
            .iter()
            .cloned()
            .fold(zero, |acc, val| acc + val)
            / len
    }
}

impl Default for Calibration {
    fn default() -> Self {
        Calibration::with_capacity(3 * 250 as usize)
    }
}
