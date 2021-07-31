use std::any::Any;

use anyhow::Result;
use cgmath::{Deg, Euler, Vector2, Vector3};
use enum_map::{Enum, EnumMap};
use hidapi::{DeviceInfo, HidApi};

#[derive(Enum, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum JoyKey {
    Up,
    Down,
    Left,
    Right,
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

impl From<bool> for KeyStatus {
    fn from(b: bool) -> Self {
        if b {
            KeyStatus::Pressed
        } else {
            KeyStatus::Released
        }
    }
}

#[derive(Debug, Clone)]
pub struct Report {
    pub keys: EnumMap<JoyKey, KeyStatus>,
    pub left_joystick: Vector2<f64>,
    pub right_joystick: Vector2<f64>,
    pub motion: Vec<Motion>,
    pub frequency: u32,
}

#[derive(Debug, Clone)]
pub struct Motion {
    pub rotation_speed: Euler<Deg<f64>>,
    pub acceleration: Vector3<f64>,
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
    fn as_any(&mut self) -> &mut dyn Any;
}
