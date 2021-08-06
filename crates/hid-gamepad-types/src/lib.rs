use std::{ops::Mul, time::Duration};

use cgmath::{vec3, Deg, Euler, Vector2, Vector3};
use enum_map::{Enum, EnumMap};

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

#[derive(Debug, Clone, Copy)]
pub struct Motion {
    pub rotation_speed: RotationSpeed,
    pub acceleration: Acceleration,
}

/// Uses the SDL convention.
///
/// Units are deg/s
#[derive(Debug, Clone, Copy)]
pub struct RotationSpeed {
    /// -x ... +x is left ... right
    pub x: f64,
    /// -y ... +y is down ... up
    pub y: f64,
    /// -z ... +z is forward ... backward
    pub z: f64,
}

impl RotationSpeed {
    pub fn as_vec(self) -> Vector3<f64> {
        vec3(self.x, self.y, self.z)
    }
}

impl From<Vector3<f64>> for RotationSpeed {
    fn from(raw: Vector3<f64>) -> Self {
        Self {
            x: raw.x,
            y: raw.y,
            z: raw.z,
        }
    }
}

impl Mul<Duration> for RotationSpeed {
    type Output = Euler<Deg<f64>>;

    fn mul(self, dt: Duration) -> Self::Output {
        Euler::new(
            Deg(self.x * dt.as_secs_f64()),
            Deg(self.y * dt.as_secs_f64()),
            Deg(self.z * dt.as_secs_f64()),
        )
    }
}

/// Uses the SDL convention.
///
/// Units are in g
#[derive(Debug, Clone, Copy)]
pub struct Acceleration {
    /// -x ... +x is left ... right
    pub x: f64,
    /// -y ... +y is down ... up
    pub y: f64,
    /// -z ... +z is forward ... backward
    pub z: f64,
}

impl Acceleration {
    pub fn as_vec(self) -> Vector3<f64> {
        vec3(self.x, self.y, self.z)
    }
}

impl From<Vector3<f64>> for Acceleration {
    fn from(raw: Vector3<f64>) -> Self {
        Self {
            x: raw.x,
            y: raw.y,
            z: raw.z,
        }
    }
}
