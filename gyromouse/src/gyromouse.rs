use cgmath::{Vector2, Zero};
use std::{collections::VecDeque, time::Duration};

use crate::config::settings::GyroSettings;

pub struct GyroMouse {
    /// Enables smoothing for slow movements.
    ///
    /// <http://gyrowiki.jibbsmart.com/blog:good-gyro-controls-part-1:the-gyro-is-a-mouse#toc8>
    pub apply_smoothing: bool,
    /// Smoothing threshold.
    ///
    /// Rotations smaller than this will be smoothed over a small period of time.
    pub smooth_threshold: f64,
    smooth_buffer: VecDeque<Vector2<f64>>,

    /// Stabilize slow movements
    ///
    /// <http://gyrowiki.jibbsmart.com/blog:good-gyro-controls-part-1:the-gyro-is-a-mouse#toc9>
    pub apply_tightening: bool,
    /// Tightening threshold.
    ///
    /// Rotations smaller than this will have smaller amplitude.
    pub tightening_threshold: f64,

    /// Enables acceleration.
    ///
    /// <http://gyrowiki.jibbsmart.com/blog:good-gyro-controls-part-1:the-gyro-is-a-mouse#toc7>
    pub apply_acceleration: bool,
    pub acceleration_slow_sens: f64,
    pub acceleration_slow_threshold: f64,
    pub acceleration_fast_sens: f64,
    pub acceleration_fast_threshold: f64,

    /// Sensitivity to use without acceleration.
    ///
    /// <http://gyrowiki.jibbsmart.com/blog:good-gyro-controls-part-1:the-gyro-is-a-mouse#toc5>
    pub sensitivity: f64,
}

impl GyroMouse {
    /// Nothing applied.
    #[allow(dead_code)]
    pub fn blank() -> GyroMouse {
        GyroMouse {
            apply_smoothing: false,
            smooth_threshold: 5.,
            smooth_buffer: VecDeque::new(),

            apply_tightening: false,
            tightening_threshold: 5.,

            apply_acceleration: false,
            acceleration_slow_sens: 8.,
            acceleration_slow_threshold: 5.,
            acceleration_fast_sens: 16.,
            acceleration_fast_threshold: 75.,

            sensitivity: 1.,
        }
    }
    /// Good default values for a 2D mouse.
    #[allow(dead_code)]
    pub fn d2() -> GyroMouse {
        GyroMouse {
            apply_smoothing: true,
            smooth_threshold: 5.,
            smooth_buffer: [Vector2::zero(); 25].iter().cloned().collect(),

            apply_tightening: true,
            tightening_threshold: 5.,

            apply_acceleration: true,
            acceleration_slow_sens: 16.,
            acceleration_slow_threshold: 5.,
            acceleration_fast_sens: 32.,
            acceleration_fast_threshold: 75.,

            sensitivity: 32.,
        }
    }

    /// Good default values for a 3D mouse.
    #[allow(dead_code)]
    pub fn d3() -> GyroMouse {
        GyroMouse {
            apply_smoothing: false,
            smooth_threshold: 0.,
            smooth_buffer: VecDeque::new(),

            apply_tightening: false,
            tightening_threshold: 0.,

            apply_acceleration: true,
            acceleration_slow_sens: 1.,
            acceleration_slow_threshold: 0.,
            acceleration_fast_sens: 2.,
            acceleration_fast_threshold: 75.,

            sensitivity: 1.,
        }
    }

    /// Process a new gyro sample.
    ///
    /// Parameter is pitch + yaw.
    ///
    /// Updates `self.orientation` and returns the applied change.
    ///
    /// `orientation` and return value have origin in bottom left.
    pub fn process(&mut self, mut rot: Vector2<f64>, dt: Duration) -> Vector2<f64> {
        if self.apply_smoothing {
            rot = self.tiered_smooth(rot);
        }
        if self.apply_tightening {
            rot = self.tight(rot);
        }
        let sens = self.get_sens(rot);
        rot * sens * dt.as_secs_f64()
    }

    fn tiered_smooth(&mut self, rot: Vector2<f64>) -> Vector2<f64> {
        let thresh_high = self.smooth_threshold;
        let thresh_low = thresh_high / 2.;
        let magnitude = (rot.x.powf(2.) + rot.y.powf(2.)).sqrt();
        let weight = ((magnitude - thresh_low) / (thresh_high - thresh_low))
            .max(0.)
            .min(1.);
        let smoothed = self.smooth(rot * (1. - weight));
        rot * weight + smoothed
    }

    fn smooth(&mut self, rot: Vector2<f64>) -> Vector2<f64> {
        self.smooth_buffer.pop_back();
        self.smooth_buffer.push_front(rot);
        let sum = self
            .smooth_buffer
            .iter()
            .fold(Vector2::zero(), |acc, x| acc + x);
        sum / self.smooth_buffer.len() as f64
    }

    fn tight(&mut self, rot: Vector2<f64>) -> Vector2<f64> {
        let magnitude = (rot.x.powf(2.) + rot.y.powf(2.)).sqrt();
        if magnitude < self.tightening_threshold {
            let scale = magnitude / self.tightening_threshold;
            rot * scale
        } else {
            rot
        }
    }

    fn get_sens(&self, rot: Vector2<f64>) -> f64 {
        if self.apply_acceleration {
            let magnitude = (rot.x.powf(2.) + rot.y.powf(2.)).sqrt();
            let factor = ((magnitude - self.acceleration_slow_threshold)
                / (self.acceleration_fast_threshold - self.acceleration_slow_threshold))
                .max(0.)
                .min(1.);
            self.acceleration_slow_sens * (1. - factor) + self.acceleration_fast_sens * factor
        } else {
            self.sensitivity
        }
    }
}

impl From<GyroSettings> for GyroMouse {
    #[allow(clippy::float_cmp)]
    fn from(settings: GyroSettings) -> Self {
        assert_eq!(settings.cutoff_speed, 0.);
        Self {
            apply_smoothing: settings.smooth_threshold != 0.,
            smooth_threshold: settings.smooth_threshold,
            // TODO
            smooth_buffer: [Vector2::zero(); 25].iter().cloned().collect(),
            apply_tightening: settings.cutoff_recovery != 0.,
            tightening_threshold: settings.cutoff_recovery,
            apply_acceleration: settings.slow_sens != 0. || settings.fast_sens != 0.,
            acceleration_slow_sens: settings.slow_sens,
            acceleration_slow_threshold: settings.slow_threshold,
            acceleration_fast_sens: settings.fast_sens,
            acceleration_fast_threshold: settings.fast_threshold,
            sensitivity: settings.sens,
        }
    }
}
