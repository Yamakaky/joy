use std::time::Duration;

use cgmath::Deg;

use crate::joystick::{ButtonStick, CameraStick, FlickStick, Stick};

use super::types::{
    AimStickSetting, FlickStickSetting, GyroSetting, GyroSpace, RingMode, Setting, StickMode,
    StickSetting, TriggerMode,
};

#[derive(Debug, Clone)]
pub struct Settings {
    pub gyro: GyroSettings,
    pub stick_settings: StickSettings,
    pub left_stick_mode: StickMode,
    pub right_stick_mode: StickMode,
    pub left_ring_mode: RingMode,
    pub right_ring_mode: RingMode,
    pub trigger_threshold: f64,
    pub zl_mode: TriggerMode,
    pub zr_mode: TriggerMode,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            gyro: GyroSettings::default(),
            stick_settings: StickSettings::default(),
            left_stick_mode: StickMode::NoMouse,
            right_stick_mode: StickMode::Aim,
            left_ring_mode: RingMode::Outer,
            right_ring_mode: RingMode::Outer,
            trigger_threshold: 0.5,
            zl_mode: TriggerMode::NoFull,
            zr_mode: TriggerMode::NoFull,
        }
    }
}

impl Settings {
    pub fn apply(&mut self, setting: Setting) {
        match setting {
            Setting::Gyro(s) => self.gyro.apply(s),
            Setting::StickSetting(s) => self.stick_settings.apply(s),
            Setting::LeftStickMode(m) => self.left_stick_mode = m,
            Setting::RightStickMode(m) => self.right_stick_mode = m,
            Setting::LeftRingMode(m) => self.left_ring_mode = m,
            Setting::RightRingMode(m) => self.right_ring_mode = m,
            Setting::TriggerThreshold(t) => self.trigger_threshold = t,
            Setting::ZLMode(m) => self.zl_mode = m,
            Setting::ZRMode(m) => self.zr_mode = m,
        }
    }

    pub fn new_left_stick(&self) -> Box<dyn Stick> {
        self.new_stick(self.left_stick_mode, true)
    }

    pub fn new_right_stick(&self) -> Box<dyn Stick> {
        self.new_stick(self.right_stick_mode, false)
    }

    fn new_stick(&self, mode: StickMode, left: bool) -> Box<dyn Stick> {
        match mode {
            StickMode::Aim => Box::new(CameraStick::default()),
            StickMode::Flick | StickMode::FlickOnly | StickMode::RotateOnly => {
                let flick = mode != StickMode::RotateOnly;
                let rotate = mode != StickMode::FlickOnly;
                Box::new(FlickStick::new(
                    &self.stick_settings.flick_stick,
                    self.stick_settings.deadzone,
                    flick,
                    rotate,
                ))
            }
            StickMode::MouseRing => todo!(),
            StickMode::MouseArea => todo!(),
            StickMode::NoMouse => Box::new(if left {
                ButtonStick::left(self.left_ring_mode)
            } else {
                ButtonStick::right(self.right_ring_mode)
            }),
            StickMode::ScrollWheel => todo!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StickSettings {
    deadzone: f64,
    fullzone: f64,
    aim_stick: AimStickSettings,
    flick_stick: FlickStickSettings,
}

impl Default for StickSettings {
    fn default() -> Self {
        Self {
            deadzone: 0.15,
            fullzone: 0.9,
            aim_stick: Default::default(),
            flick_stick: Default::default(),
        }
    }
}

impl StickSettings {
    fn apply(&mut self, setting: StickSetting) {
        match setting {
            StickSetting::Deadzone(d) => self.deadzone = d,
            StickSetting::FullZone(d) => self.fullzone = d,
            StickSetting::Aim(s) => self.aim_stick.apply(s),
            StickSetting::Flick(s) => self.flick_stick.apply(s),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AimStickSettings {
    sens: f64,
    power: f64,
    invert_x: bool,
    invert_y: bool,
    acceleration_rate: f64,
    acceleration_cap: f64,
}

impl Default for AimStickSettings {
    fn default() -> Self {
        Self {
            sens: 360.,
            power: 1.,
            invert_x: false,
            invert_y: false,
            acceleration_rate: 0.,
            acceleration_cap: 1000000.,
        }
    }
}

impl AimStickSettings {
    fn apply(&mut self, setting: AimStickSetting) {
        match setting {
            AimStickSetting::Sens(s) => self.sens = s,
            AimStickSetting::Power(s) => self.power = s,
            AimStickSetting::InvertX => self.invert_x = true,
            AimStickSetting::InvertY => self.invert_y = true,
            AimStickSetting::AccelerationRate(s) => self.acceleration_rate = s,
            AimStickSetting::AccelerationCap(s) => self.acceleration_cap = s,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FlickStickSettings {
    pub flick_time: Duration,
    pub exponent: f64,
    pub forward_deadzone_arc: Deg<f64>,
}

impl Default for FlickStickSettings {
    fn default() -> Self {
        Self {
            flick_time: Duration::from_millis(100),
            exponent: 0.,
            forward_deadzone_arc: Deg(0.),
        }
    }
}

impl FlickStickSettings {
    fn apply(&mut self, setting: FlickStickSetting) {
        match setting {
            FlickStickSetting::FlickTime(s) => self.flick_time = s,
            FlickStickSetting::Exponent(s) => self.exponent = s,
            FlickStickSetting::ForwardDeadzoneArc(s) => self.forward_deadzone_arc = s,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GyroSettings {
    pub sens: f64,
    pub space: GyroSpace,
    pub cutoff_speed: f64,
    pub cutoff_recovery: f64,
    pub smooth_threshold: f64,
    pub smooth_time: Duration,
    pub slow_threshold: f64,
    pub slow_sens: f64,
    pub fast_threshold: f64,
    pub fast_sens: f64,
}

impl Default for GyroSettings {
    fn default() -> Self {
        Self {
            sens: 1.,
            space: GyroSpace::Local,
            cutoff_speed: 0.,
            cutoff_recovery: 0.,
            smooth_threshold: 0.,
            smooth_time: Duration::from_millis(125),
            slow_sens: 0.,
            slow_threshold: 0.,
            fast_sens: 0.,
            fast_threshold: 0.,
        }
    }
}

impl GyroSettings {
    fn apply(&mut self, setting: GyroSetting) {
        match setting {
            GyroSetting::Sensitivity(s) => self.sens = s,
            GyroSetting::MinSens(s) => self.slow_sens = s,
            GyroSetting::MinThreshold(s) => self.slow_threshold = s,
            GyroSetting::MaxSens(s) => self.fast_sens = s,
            GyroSetting::MaxThreshold(s) => self.fast_threshold = s,
            GyroSetting::Space(s) => self.space = s,
            GyroSetting::CutoffSpeed(s) => self.cutoff_speed = s,
            GyroSetting::CutoffRecovery(s) => self.cutoff_recovery = s,
            GyroSetting::SmoothThreshold(s) => self.smooth_threshold = s,
            GyroSetting::SmoothTime(s) => self.smooth_time = s,
        }
    }
}
