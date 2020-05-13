use joycon_sys::common::Vector3;
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct Calibration {
    history: VecDeque<Vector3>,
    capacity: usize,
    pub factory_offset: Vector3,
    pub user_offset: Option<Vector3>,
}

impl Calibration {
    pub fn new(capacity: usize) -> Calibration {
        Calibration {
            history: VecDeque::with_capacity(capacity),
            capacity,
            factory_offset: Default::default(),
            user_offset: Default::default(),
        }
    }

    pub fn push(&mut self, entry: Vector3) {
        if self.history.len() == self.capacity {
            self.history.pop_back();
        }
        self.history.push_front(entry);
    }

    pub fn reset(&mut self) {
        self.history.clear();
    }

    pub fn get_average(&mut self) -> Vector3 {
        let len = self.history.len() as f32;
        if len == 0. {
            return Vector3(0., 0., 0.);
        }
        let sum = self
            .history
            .iter()
            .fold(Vector3(0., 0., 0.), |acc, val| acc + *val);
        Vector3(sum.0 / len, sum.1 / len, sum.2 / len)
    }
}
