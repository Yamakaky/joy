use std::fmt;

use bitfield::bitfield;
use cgmath::{vec2, vec3, Deg, Euler, Vector2, Vector3};

use crate::{RawId, DS4_REPORT_DT, I16LE};

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

    pub fn simple(&self) -> Option<&BTSimpleReport> {
        if self.id == InputReportId::Simple {
            Some(unsafe { &self.u.simple })
        } else {
            None
        }
    }

    pub fn complete(&self) -> Option<&BTFullReport> {
        if self.id == InputReportId::Complete {
            Some(unsafe { &self.u.complete })
        } else {
            None
        }
    }

    pub fn usb(&self) -> Option<&USBReport> {
        if self.id == InputReportId::Simple {
            Some(unsafe { &self.u.usb })
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
    simple: BTSimpleReport,
    complete: BTFullReport,
    usb: USBReport,
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct USBReport {
    pub full: FullReport,
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct BTSimpleReport {
    pub base: SimpleReport,
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct BTFullReport {
    _unknown1: u8,
    report_id: u8,
    pub full: FullReport,
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct FullReport {
    pub base: SimpleReport,
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
pub struct SimpleReport {
    pub left_stick: Stick,
    pub right_stick: Stick,
    pub buttons: Buttons<[u8; 3]>,
    pub left_trigger: u8,
    pub right_trigger: u8,
}

bitfield! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Default)]
    pub struct Buttons([u8]);
    impl Debug;
    u8;
    pub dpad, _: 2, 0;
    pub dpad_pressed, _: 3;
    pub square, _: 4;
    pub cross, _: 5;
    pub circle, _: 6;
    pub triangle, _: 7;
    pub l1, _: 8;
    pub r1, _: 9;
    pub l2, _: 10;
    pub r2, _: 11;
    pub share, _: 12;
    pub options, _: 13;
    pub l3, _: 14;
    pub r3, _: 15;
    pub ps, _: 16;
    pub tpad, _: 17;
    pub counter, _: 23, 18;
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct Gyro {
    pitch: I16LE,
    yaw: I16LE,
    roll: I16LE,
}

impl Gyro {
    pub fn delta(&self) -> Euler<Deg<f64>> {
        let factor = 2000. / (2.0_f64.powi(15));
        let pitch = Deg(i16::from(self.pitch) as f64 * DS4_REPORT_DT * factor);
        let yaw = Deg(-i16::from(self.yaw) as f64 * DS4_REPORT_DT * factor);
        let roll = Deg(-i16::from(self.roll) as f64 * DS4_REPORT_DT * factor);
        Euler::new(pitch, yaw, roll)
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
#[derive(Clone, Copy)]
pub struct Stick {
    x: u8,
    y: u8,
}

impl Stick {
    pub fn val(&self) -> (u8, u8) {
        (self.x, 255 - self.y)
    }

    pub fn normalize(&self) -> Vector2<f64> {
        let x = self.x as f64 - 128.;
        let y = self.y as f64 - 128.;
        vec2(x, -y) / 128.
    }
}

impl fmt::Debug for Stick {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = self.normalize();
        f.debug_tuple("Stick").field(&s.x).field(&s.y).finish()
    }
}

#[repr(packed)]
#[derive(Clone, Copy)]
pub struct Trackpad {
    len: u8,
    packets: [TrackpadPacket; 4],
}

impl Trackpad {
    pub fn packets(&self) -> impl Iterator<Item = &TrackpadPacket> {
        self.packets
            .iter()
            .take(self.len as usize)
            .filter(|p| p.fingers().any(|f| f.is_active()))
    }
}

impl fmt::Debug for Trackpad {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.packets()).finish()
    }
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct TrackpadPacket {
    counter: u8,
    fingers: [Finger; 2],
}

impl TrackpadPacket {
    pub fn fingers(&self) -> impl Iterator<Item = &Finger> {
        self.fingers.iter().filter(|f| f.is_active())
    }
}

#[repr(packed)]
#[derive(Clone, Copy)]
pub struct Finger {
    id: u8,
    coordinate: FingerCoord,
}

impl Finger {
    pub fn is_active(&self) -> bool {
        self.id & 0x80 == 0
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

#[repr(packed)]
#[derive(Clone, Copy)]
pub struct FingerCoord(u8, u8, u8);

impl FingerCoord {
    pub fn val(&self) -> (u16, u16) {
        let (a, b, c) = (self.0 as u16, self.1 as u16, self.2 as u16);
        (((b & 0xf) << 8) | a, (c << 4) | ((b & 0xf0) >> 4))
    }
}

impl fmt::Debug for FingerCoord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let val = self.val();
        f.debug_tuple("FingerCoord")
            .field(&val.0)
            .field(&val.1)
            .finish()
    }
}
