use std::fmt;

use cgmath::{vec3, Vector3};

use crate::{RawId, I16LE};

#[repr(packed)]
#[derive(Clone, Copy)]
pub struct InputReport {
    id: RawId<InputReportId>,
    u: InputReportData,
}

impl InputReport {
    pub fn new() -> InputReport {
        unsafe { std::mem::zeroed() }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self as *mut _ as *mut u8, std::mem::size_of_val(self))
        }
    }

    pub fn simple(&self) -> Option<&SimpleReport> {
        if self.id == InputReportId::Simple {
            Some(unsafe { &self.u.simple })
        } else {
            None
        }
    }

    pub fn complete(&self) -> Option<&CompleteReport> {
        if self.id == InputReportId::Complete {
            Some(unsafe { &self.u.complete })
        } else {
            None
        }
    }
}

impl fmt::Debug for InputReport {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.id.try_into() {
            Some(InputReportId::Simple) => self.simple().fmt(f),
            Some(InputReportId::Complete) => self.complete().fmt(f),
            None => unimplemented!(),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
pub enum InputReportId {
    Simple = 0x01,
    Complete = 0x11,
}

#[repr(packed)]
#[derive(Clone, Copy)]
union InputReportData {
    simple: SimpleReport,
    complete: CompleteReport,
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct SimpleReport {}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct CompleteReport {
    _unknown1: u8,
    report_id: u8,
    pub left_stick: Stick,
    pub right_stick: Stick,
    pub buttons: [u8; 3],
    pub left_trigger: u8,
    pub right_trigger: u8,
    _unknown_timestamp: [u8; 2],
    battery_level: u8,
    pub gyro: Gyro,
    pub accel: Accel,
    _unknown_0x00: [u8; 5],
    type_: u8,
    unknown_0x00_2: [u8; 2],
    pub trackpad: Trackpad,
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct Gyro {
    pitch: I16LE,
    yaw: I16LE,
    roll: I16LE,
}

impl Gyro {
    /// Yaw, pitch, roll in this order. Unit in degree per second (dps).
    pub fn val(&self) -> Vector3<i16> {
        vec3(
            -i16::from(self.yaw),
            self.pitch.into(),
            -i16::from(self.roll),
        )
    }
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct Accel {
    y: I16LE,
    z: I16LE,
    x: I16LE,
}

impl Accel {
    /// Yaw, pitch, roll in this order. Unit in degree per second (dps).
    pub fn val(&self) -> Vector3<i16> {
        vec3(i16::from(self.x), -i16::from(self.y), i16::from(self.z))
    }
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct Stick {
    x: u8,
    y: u8,
}

impl Stick {
    pub fn val(&self) -> (u8, u8) {
        (self.x, 255 - self.y)
    }
}

#[repr(packed)]
#[derive(Clone, Copy)]
pub struct Trackpad {
    len: u8,
    packets: [TrackpadPacket; 4],
}

impl Trackpad {
    pub fn packets(&self) -> &[TrackpadPacket] {
        &self.packets[..self.len as usize]
    }
}

impl fmt::Debug for Trackpad {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.packets()).finish()
    }
}

#[repr(packed)]
#[derive(Clone, Copy)]
pub struct TrackpadPacket {
    fingers: [Finger; 2],
}

impl TrackpadPacket {
    pub fn fingers(&self) -> impl Iterator<Item = &Finger> {
        self.fingers.iter().filter(|f| f.is_active())
    }
}

impl fmt::Debug for TrackpadPacket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.fingers()).finish()
    }
}

#[repr(packed)]
#[derive(Clone, Copy)]
pub struct Finger {
    id: u8,
    coordinate: [u8; 3],
}

impl Finger {
    pub fn is_active(&self) -> bool {
        self.id & 0x80 != 0
    }

    pub fn id(&self) -> u8 {
        self.id & 0x7F
    }
}

impl fmt::Debug for Finger {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_active() {
            f.debug_struct("Finger")
                .field("id", &self.id())
                .field("coordinate", &self.coordinate)
                .finish()
        } else {
            f.debug_struct("Finger (none)").finish()
        }
    }
}
