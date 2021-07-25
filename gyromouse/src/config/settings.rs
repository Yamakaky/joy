use std::time::Duration;

use cgmath::Deg;

use super::types::{
    AimStickSetting, FlickStickSetting, Setting, StickMode, StickSetting, TriggerMode,
};

#[derive(Debug, Clone)]
pub struct Settings {
    stick_settings: StickSettings,
    left_stick_mode: StickMode,
    right_stick_mode: StickMode,
    trigger_threshold: f64,
    zl_mode: TriggerMode,
    zr_mode: TriggerMode,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            stick_settings: StickSettings::default(),
            left_stick_mode: StickMode::NoMouse,
            right_stick_mode: StickMode::Aim,
            trigger_threshold: 0.5,
            zl_mode: TriggerMode::NoFull,
            zr_mode: TriggerMode::NoFull,
        }
    }
}

impl Settings {
    pub fn apply(&mut self, setting: Setting) {
        match setting {
            Setting::StickSetting(s) => self.stick_settings.apply(s),
            Setting::LeftStickMode(m) => self.left_stick_mode = m,
            Setting::RightStickMode(m) => self.right_stick_mode = m,
            Setting::TriggerThreshold(t) => self.trigger_threshold = t,
            Setting::ZLMode(m) => self.zl_mode = m,
            Setting::ZRMode(m) => self.zr_mode = m,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct StickSettings {
    aim_stick: AimStickSettings,
    flick_stick: FlickStickSettings,
}

impl StickSettings {
    fn apply(&mut self, setting: StickSetting) {
        match setting {
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
    deadzone: f64,
    fullzone: f64,
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
            deadzone: 0.15,
            fullzone: 0.1,
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
            AimStickSetting::Deadzone(s) => self.deadzone = s,
            AimStickSetting::FullZone(s) => self.fullzone = s,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FlickStickSettings {
    flick_time: Duration,
    exponent: f64,
    forward_deadzone_arc: Deg<f64>,
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
