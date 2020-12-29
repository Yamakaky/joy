use std::{fmt, mem::size_of};

use bitfield::bitfield;
use cgmath::{vec2, vec3, Deg, Euler, Vector2, Vector3};

use crate::{ConnectionType, RawId, I16LE};

#[repr(packed)]
#[derive(Clone, Copy)]
pub struct InputReport {
    id: RawId<InputReportId>,
    u: InputReportData,
    // allows detection of larger reads in conn_type
    _padding: u8,
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

    pub fn bt_simple(&self) -> Option<&BTSimpleReport> {
        if self.id == InputReportId::Simple {
            Some(unsafe { &self.u.simple })
        } else {
            None
        }
    }

    pub fn bt_full(&self) -> Option<&BTFullReport> {
        if self.id == InputReportId::Full {
            Some(unsafe { &self.u.complete })
        } else {
            None
        }
    }

    pub fn usb_full(&self) -> Option<&USBReport> {
        // USB uses different ids smh...
        if self.id == InputReportId::Simple {
            Some(unsafe { &self.u.usb })
        } else {
            None
        }
    }

    pub fn conn_type(mut nb_read: usize) -> ConnectionType {
        // Remove report id
        nb_read -= 1;
        // TODO: better detection, what about other reports?
        if nb_read == size_of::<USBReport>() {
            ConnectionType::USB
        } else if [size_of::<BTSimpleReport>(), size_of::<BTFullReport>()].contains(&nb_read) {
            ConnectionType::Bluetooth
        } else {
            dbg!(size_of::<USBReport>());
            dbg!(nb_read);
            unreachable!()
        }
    }
}

impl fmt::Debug for InputReport {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO: handle bluetooth
        match self.id.try_into() {
            Some(InputReportId::Simple) => self.bt_simple().fmt(f),
            Some(InputReportId::Full) => self.bt_full().fmt(f),
            None => unimplemented!(),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
pub enum InputReportId {
    Simple = 0x01,
    Full = 0x11,
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
    pub trackpad: USBTrackpad,
    _unknown: [u8; 12],
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
    pub trackpad: BTTrackpad,
    _unknown2: [u8; 2],
    crc32: [u8; 4],
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
    pub type_: Type,
    unknown_0x00_2: [u8; 2],
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct SimpleReport {
    pub left_stick: Stick,
    pub right_stick: Stick,
    pub buttons: Buttons<[u8; 3]>,
    pub left_trigger: Trigger,
    pub right_trigger: Trigger,
}

bitfield! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Default)]
    pub struct Type(u8);
    impl Debug;
    u8;
    pub battery, _: 3, 0;
    pub usb, _: 4;
    pub mic, _: 5;
    pub phone, _: 6;
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Trigger(u8);

impl Trigger {
    pub fn normalize(&self) -> f64 {
        self.0 as f64 / 255.
    }
}

impl fmt::Debug for Trigger {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Trigger").field(&self.normalize()).finish()
    }
}

bitfield! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Default)]
    pub struct Buttons([u8]);
    impl Debug;
    u8;
    pub into Dpad, dpad, _: 2, 0;
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

#[derive(Debug, Clone, Copy)]
pub struct Dpad(u8);

impl Dpad {
    pub fn up(&self) -> bool {
        self.0 <= 1 || self.0 == 7
    }

    pub fn right(&self) -> bool {
        1 <= self.0 && self.0 <= 3
    }

    pub fn down(&self) -> bool {
        3 <= self.0 && self.0 <= 5
    }

    pub fn left(&self) -> bool {
        5 <= self.0 && self.0 <= 7
    }
}

impl From<u8> for Dpad {
    fn from(u: u8) -> Self {
        Dpad(u)
    }
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct Gyro {
    pitch: I16LE,
    yaw: I16LE,
    roll: I16LE,
}

impl Gyro {
    pub fn normalize(&self) -> Euler<Deg<f64>> {
        let factor = 2000. / (2.0_f64.powi(15));
        let pitch = Deg(i16::from(self.pitch) as f64 * factor);
        let yaw = Deg(-i16::from(self.yaw) as f64 * factor);
        let roll = Deg(-i16::from(self.roll) as f64 * factor);
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
    pub fn raw(&self) -> Vector3<i16> {
        vec3(i16::from(self.x), -i16::from(self.y), i16::from(self.z))
    }

    /// Convert to SI units, in G across each axis.
    pub fn normalize(&self) -> Vector3<f64> {
        self.raw().cast::<f64>().unwrap() / 2.0_f64.powi(15) * 4.
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
pub struct BTTrackpad {
    len: u8,
    packets: [TrackpadPacket; 4],
}

impl BTTrackpad {
    pub fn packets(&self) -> impl Iterator<Item = &TrackpadPacket> {
        self.packets
            .iter()
            .take(self.len as usize)
            .filter(|p| p.fingers().any(|f| f.is_active()))
    }
}

impl fmt::Debug for BTTrackpad {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.packets()).finish()
    }
}

#[repr(packed)]
#[derive(Clone, Copy)]
pub struct USBTrackpad {
    _unknown: u8,
    packets: [TrackpadPacket; 2],
}

impl USBTrackpad {
    pub fn packets(&self) -> impl Iterator<Item = &TrackpadPacket> {
        self.packets
            .iter()
            .filter(|p| p.fingers().any(|f| f.is_active()))
    }
}

impl fmt::Debug for USBTrackpad {
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

    pub fn coord(&self) -> Vector2<f64> {
        self.coordinate.normalize()
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
    pub fn raw(&self) -> (u16, u16) {
        let (a, b, c) = (self.0 as u16, self.1 as u16, self.2 as u16);
        (((b & 0xf) << 8) | a, (c << 4) | ((b & 0xf0) >> 4))
    }

    pub fn normalize(&self) -> Vector2<f64> {
        let (x, y) = self.raw();
        vec2(x as f64 / 1919., y as f64 / 942.)
    }
}

impl fmt::Debug for FingerCoord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let val = self.raw();
        f.debug_tuple("FingerCoord")
            .field(&val.0)
            .field(&val.1)
            .finish()
    }
}
