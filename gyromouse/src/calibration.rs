use cgmath::{num_traits::zero, MetricSpace, Vector3};
use hid_gamepad_types::{Acceleration, Motion, RotationSpeed};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy)]
pub struct Calibration {
    gyro: Vector3<f64>,
}

impl Calibration {
    pub fn empty() -> Self {
        Self { gyro: zero() }
    }

    pub fn calibrate(&self, mut motion: Motion) -> Motion {
        motion.rotation_speed = (motion.rotation_speed.as_vec() - self.gyro).into();
        motion
    }
}

type Entry = Vector3<f64>;

#[derive(Clone, Debug)]
pub struct SimpleCalibration {
    history: VecDeque<Entry>,
    capacity: usize,
}

impl SimpleCalibration {
    pub fn with_capacity(capacity: usize) -> SimpleCalibration {
        SimpleCalibration {
            history: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    #[allow(dead_code)]
    pub fn push(&mut self, entry: Entry) {
        if self.history.len() == self.capacity {
            self.history.pop_back();
        }
        self.history.push_front(entry);
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.history.clear();
    }

    #[allow(dead_code)]
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

impl Default for SimpleCalibration {
    fn default() -> Self {
        SimpleCalibration::with_capacity(3 * 250_usize)
    }
}

#[derive(Debug, Clone)]
pub struct BetterCalibration {
    last_sum: Vector3<f64>,
    last_count: u64,
    total_nb_samples: u64,
    last: Motion,
    state: BetterCalibrationState,
}

#[derive(Debug, Clone)]
enum BetterCalibrationState {
    Moving,
    Static {
        sum: Vector3<f64>,
        count: u64,
        start: Instant,
    },
}

impl BetterCalibration {
    pub fn push(&mut self, motion: Motion, now: Instant, limit: Duration) -> bool {
        let rot_dist = self
            .last
            .rotation_speed
            .as_vec()
            .distance(motion.rotation_speed.as_vec());
        let acc_dist = self
            .last
            .acceleration
            .as_vec()
            .distance(motion.acceleration.as_vec());
        let is_static = rot_dist < 1. && acc_dist < 0.01;
        let mut finished = false;
        if is_static {
            match self.state {
                BetterCalibrationState::Moving => {
                    self.state = BetterCalibrationState::Static {
                        sum: zero(),
                        count: 0,
                        start: now,
                    }
                }
                BetterCalibrationState::Static {
                    ref mut sum,
                    ref mut count,
                    start,
                } => {
                    *sum += motion.rotation_speed.as_vec();
                    *count += 1;
                    if now.duration_since(start) >= limit {
                        finished = true;
                    }
                }
            }
        } else if let BetterCalibrationState::Static { sum, count, .. } = self.state {
            if count > self.last_count {
                self.last_sum = sum;
                self.last_count = count;
            }
            self.state = BetterCalibrationState::Moving;
        }
        self.last = motion;
        self.total_nb_samples += 1;
        finished
    }

    pub fn finish(mut self) -> Calibration {
        if let BetterCalibrationState::Static { sum, count, .. } = self.state {
            if self.last_count < count {
                self.last_count = count;
                self.last_sum = sum;
            }
        }
        Calibration {
            gyro: self.last_sum / self.last_count as f64,
        }
    }
}

impl Default for BetterCalibration {
    fn default() -> Self {
        Self {
            last_sum: zero(),
            last_count: 0,
            last: Motion {
                rotation_speed: RotationSpeed {
                    x: 0.,
                    y: 0.,
                    z: 0.,
                },
                acceleration: Acceleration {
                    x: 0.,
                    y: 0.,
                    z: 0.,
                },
            },
            total_nb_samples: 0,
            state: BetterCalibrationState::Moving,
        }
    }
}
