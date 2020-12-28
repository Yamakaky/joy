use anyhow::Result;
use cgmath::{Vector2, Zero};
use enum_map::{Enum, EnumMap};
use hidapi::{DeviceInfo, HidApi};

#[derive(Enum, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    LUp,
    LDown,
    LLeft,
    LRight,
    RUp,
    RDown,
    RLeft,
    RRight,
    N,
    S,
    E,
    W,
    L,
    R,
    ZL,
    ZR,
    SL,
    SR,
    L3,
    R3,
    Minus,
    Plus,
    Capture,
    Home,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum KeyStatus {
    Pressed,
    Released,
}

impl Default for KeyStatus {
    fn default() -> Self {
        KeyStatus::Released
    }
}

#[derive(Debug, Clone)]
pub struct Report {
    pub keys: EnumMap<Key, KeyStatus>,
    pub left_joystick: Vector2<f64>,
    pub right_joystick: Vector2<f64>,
}

impl Report {
    pub fn new() -> Report {
        Report {
            keys: EnumMap::default(),
            left_joystick: Vector2::zero(),
            right_joystick: Vector2::zero(),
        }
    }
}

pub trait GamepadDriver {
    fn init(
        &self,
        api: &HidApi,
        device_info: &DeviceInfo,
    ) -> Result<Option<Box<dyn GamepadDevice>>>;
}

pub trait GamepadDevice {
    fn recv(&mut self) -> Result<Report>;
}
