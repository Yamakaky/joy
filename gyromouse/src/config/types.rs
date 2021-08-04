use std::time::Duration;

use cgmath::Deg;

use crate::{
    mapping::{ExtAction, MapKey},
    ClickType,
};

#[derive(Debug, Copy, Clone)]
pub enum ActionModifier {
    Toggle,
    Instant,
}

#[derive(Debug, Copy, Clone)]
pub enum EventModifier {
    Tap,
    Hold,
    Start,
    Release,
    Turbo,
}

#[derive(Debug, Copy, Clone)]
pub struct JSMAction {
    pub action_mod: Option<ActionModifier>,
    pub event_mod: Option<EventModifier>,
    pub action: ActionType,
}

#[derive(Debug, Copy, Clone)]
pub enum ActionType {
    Key(enigo::Key),
    Mouse(enigo::MouseButton),
    Special(SpecialKey),
}

impl From<(ActionType, ClickType)> for ExtAction {
    fn from((a, b): (ActionType, ClickType)) -> Self {
        match a {
            ActionType::Key(k) => ExtAction::KeyPress(k, b),
            ActionType::Mouse(k) => ExtAction::MousePress(k, b),
            ActionType::Special(SpecialKey::GyroOn) => ExtAction::GyroOn(b),
            ActionType::Special(SpecialKey::GyroOff) => ExtAction::GyroOff(b),
            ActionType::Special(SpecialKey::None) => unimplemented!(),
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Key {
    Simple(MapKey),
    Simul(MapKey, MapKey),
    Chorded(MapKey, MapKey),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SpecialKey {
    None,
    GyroOn,
    GyroOff,
    GyroInvertX(bool),
    GyroInvertY(bool),
    GyroTrackBall(bool),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TriggerMode {
    NoFull,
    NoSkip,
    NoSkipExclusive,
    MustSkip,
    MaySkip,
    MustSkipR,
    MaySkipR,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum StickMode {
    Aim,
    Flick,
    FlickOnly,
    RotateOnly,
    MouseRing,
    MouseArea,
    NoMouse,
    ScrollWheel,
}

#[derive(Debug, Copy, Clone)]
pub enum StickSetting {
    Deadzone(f64),
    FullZone(f64),
    Aim(AimStickSetting),
    Flick(FlickStickSetting),
    Scroll(ScrollStickSetting),
}

#[derive(Debug, Copy, Clone)]
pub enum AimStickSetting {
    Sens(f64),
    Power(f64),
    InvertX(bool),
    InvertY(bool),
    AccelerationRate(f64),
    AccelerationCap(f64),
}

#[derive(Debug, Copy, Clone)]
pub enum FlickStickSetting {
    FlickTime(Duration),
    Exponent(f64),
    ForwardDeadzoneArc(Deg<f64>),
}

#[derive(Debug, Copy, Clone)]
pub enum ScrollStickSetting {
    Sens(Deg<f64>),
}

#[derive(Debug, Copy, Clone)]
pub enum GyroSetting {
    Sensitivity(f64),
    MinSens(f64),
    MinThreshold(f64),
    MaxSens(f64),
    MaxThreshold(f64),
    Space(GyroSpace),
    CutoffSpeed(f64),
    CutoffRecovery(f64),
    SmoothThreshold(f64),
    SmoothTime(Duration),
}

#[derive(Debug, Copy, Clone)]
pub enum GyroSpace {
    Local,
    WorldTurn,
    WorldLean,
    PlayerTurn,
    PlayerLean,
}

#[derive(Debug, Copy, Clone)]
pub enum Setting {
    Gyro(GyroSetting),
    TriggerThreshold(f64),
    ZLMode(TriggerMode),
    ZRMode(TriggerMode),
    LeftStickMode(StickMode),
    RightStickMode(StickMode),
    LeftRingMode(RingMode),
    RightRingMode(RingMode),
    Stick(StickSetting),
}

#[derive(Debug, Clone)]
pub enum Cmd {
    Map(Key, Vec<JSMAction>),
    Special(SpecialKey),
    Setting(Setting),
    Reset,
    RealWorldCalibration(f64),
    InGameSens(f64),
    CounterOSMouseSpeed(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingMode {
    Inner,
    Outer,
}
