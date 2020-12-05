use std::collections::VecDeque;

pub struct GyroMouse {
    /// Accumulated orientation
    pub orientation: (f64, f64),

    /// Enables smoothing for slow movements.
    ///
    /// http://gyrowiki.jibbsmart.com/blog:good-gyro-controls-part-1:the-gyro-is-a-mouse#toc8
    pub apply_smoothing: bool,
    /// Smoothing threshold.
    ///
    /// Rotations smaller than this will be smoothed over a small period of time.
    pub smooth_threshold: f64,
    smooth_buffer: VecDeque<(f64, f64)>,

    /// Stabilize slow movements
    ///
    /// http://gyrowiki.jibbsmart.com/blog:good-gyro-controls-part-1:the-gyro-is-a-mouse#toc9
    pub apply_tightening: bool,
    /// Tightening threshold.
    ///
    /// Rotations smaller than this will have smaller amplitude.
    pub tightening_threshold: f64,

    /// Enables acceleration.
    ///
    /// http://gyrowiki.jibbsmart.com/blog:good-gyro-controls-part-1:the-gyro-is-a-mouse#toc7
    pub apply_acceleration: bool,
    pub acceleration_slow_sens: f64,
    pub acceleration_slow_threshold: f64,
    pub acceleration_fast_sens: f64,
    pub acceleration_fast_threshold: f64,

    /// Sensitivity to use without acceleration.
    ///
    /// http://gyrowiki.jibbsmart.com/blog:good-gyro-controls-part-1:the-gyro-is-a-mouse#toc5
    pub sensitivity: f64,
}

impl GyroMouse {
    /// Good default values for a 2D mouse.
    pub fn d2() -> GyroMouse {
        GyroMouse {
            orientation: (0., 0.),

            apply_smoothing: true,
            smooth_threshold: 5.,
            smooth_buffer: [(0., 0.); 10].iter().cloned().collect(),

            apply_tightening: true,
            tightening_threshold: 5.,

            apply_acceleration: true,
            acceleration_slow_sens: 8.,
            acceleration_slow_threshold: 5.,
            acceleration_fast_sens: 16.,
            acceleration_fast_threshold: 75.,

            sensitivity: 16.,
        }
    }

    /// Good default values for a 3D mouse.
    pub fn d3() -> GyroMouse {
        GyroMouse {
            orientation: (0., 0.),

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
    /// Updates `self.orientation` and returns the applied change.
    pub fn process(&mut self, mut rot: (f64, f64), dt: f64) -> (f64, f64) {
        if self.apply_smoothing {
            rot = self.tiered_smooth(rot);
        }
        if self.apply_tightening {
            rot = self.tight(rot);
        }
        let sens = self.get_sens(rot);
        self.orientation.0 += rot.0 * sens * dt;
        self.orientation.1 += rot.1 * sens * dt;
        (rot.0 * sens * dt, rot.1 * sens * dt)
    }

    fn tiered_smooth(&mut self, rot: (f64, f64)) -> (f64, f64) {
        let thresh_high = self.smooth_threshold;
        let thresh_low = thresh_high / 2.;
        let magnitude = (rot.0.powf(2.) + rot.1.powf(2.)).sqrt();
        let weight = ((magnitude - thresh_low) / (thresh_high - thresh_low))
            .max(0.)
            .min(1.);
        let smoothed = self.smooth((rot.0 * (1. - weight), rot.1 * (1. - weight)));
        (rot.0 * weight + smoothed.0, rot.1 * weight + smoothed.1)
    }

    fn smooth(&mut self, rot: (f64, f64)) -> (f64, f64) {
        self.smooth_buffer.pop_back();
        self.smooth_buffer.push_front(rot);
        let sum = self
            .smooth_buffer
            .iter()
            .fold((0., 0.), |acc, x| (acc.0 + x.0, acc.1 + x.1));
        (
            sum.0 / self.smooth_buffer.len() as f64,
            sum.1 / self.smooth_buffer.len() as f64,
        )
    }

    fn tight(&mut self, rot: (f64, f64)) -> (f64, f64) {
        let magnitude = (rot.0.powf(2.) + rot.1.powf(2.)).sqrt();
        if magnitude < self.tightening_threshold {
            let scale = magnitude / self.tightening_threshold;
            (rot.0 * scale, rot.1 * scale)
        } else {
            rot
        }
    }

    fn get_sens(&self, rot: (f64, f64)) -> f64 {
        if self.apply_acceleration {
            let magnitude = (rot.0.powf(2.) + rot.1.powf(2.)).sqrt();
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
