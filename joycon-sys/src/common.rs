use byteorder::{ByteOrder, LittleEndian};
use cgmath::Vector3;
use num::{FromPrimitive, ToPrimitive};
use std::fmt;
use std::marker::PhantomData;

pub const NINTENDO_VENDOR_ID: u16 = 1406;

pub const JOYCON_L_BT: u16 = 0x2006;
pub const JOYCON_R_BT: u16 = 0x2007;
pub const PRO_CONTROLLER: u16 = 0x2009;
pub const JOYCON_CHARGING_GRIP: u16 = 0x200e;

#[repr(u8)]
#[derive(Copy, Clone, Debug, FromPrimitive, ToPrimitive, PartialEq, Eq)]
pub enum InputReportId {
    Normal = 0x3F,
    StandardAndSubcmd = 0x21,
    MCUFwUpdate = 0x23,
    StandardFull = 0x30,
    StandardFullMCU = 0x31,
    // 0x32 not used
    // 0x33 not used
}

// All unused values are a Nop
#[repr(u8)]
#[derive(Copy, Clone, Debug, FromPrimitive, ToPrimitive, PartialEq, Eq)]
pub enum SubcommandId {
    GetOnlyControllerState = 0x00,
    BluetoothManualPairing = 0x01,
    RequestDeviceInfo = 0x02,
    SetInputReportMode = 0x03,
    SPIRead = 0x10,
    SPIWrite = 0x11,
    SetMCUConf = 0x21,
    SetMCUState = 0x22,
    SetPlayerLights = 0x30,
    SetHomeLight = 0x38,
    EnableIMU = 0x40,
    SetIMUSens = 0x41,
}

#[derive(Copy, Clone, Default, Eq, PartialEq)]
pub struct U16LE([u8; 2]);

impl From<u16> for U16LE {
    fn from(u: u16) -> Self {
        let mut val = [0; 2];
        LittleEndian::write_u16(&mut val, u);
        U16LE(val)
    }
}

impl From<U16LE> for u16 {
    fn from(u: U16LE) -> u16 {
        LittleEndian::read_u16(&u.0)
    }
}

impl fmt::Debug for U16LE {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        u16::from(*self).fmt(f)
    }
}

#[derive(Copy, Clone, Default, Eq, PartialEq)]
pub struct I16LE([u8; 2]);

impl From<i16> for I16LE {
    fn from(u: i16) -> I16LE {
        let mut val = [0; 2];
        LittleEndian::write_i16(&mut val, u);
        I16LE(val)
    }
}

impl From<I16LE> for i16 {
    fn from(u: I16LE) -> i16 {
        LittleEndian::read_i16(&u.0)
    }
}

impl fmt::Debug for I16LE {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        i16::from(*self).fmt(f)
    }
}

#[cfg(test)]
pub(crate) fn offset_of<A, B>(a: &A, b: &B) -> usize {
    b as *const _ as usize - a as *const _ as usize
}

pub fn vector_from_raw(raw: [I16LE; 3]) -> Vector3<f64> {
    Vector3::new(
        i16::from(raw[0]) as f64,
        i16::from(raw[1]) as f64,
        i16::from(raw[2]) as f64,
    )
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct RawId<Id>(u8, PhantomData<Id>);

impl<Id: FromPrimitive> RawId<Id> {
    pub fn try_into(self) -> Option<Id> {
        Id::from_u8(self.0)
    }
}

impl<Id: ToPrimitive> From<Id> for RawId<Id> {
    fn from(id: Id) -> Self {
        RawId(id.to_u8().expect("always one byte"), PhantomData)
    }
}

impl<Id: fmt::Debug + FromPrimitive + Copy> fmt::Debug for RawId<Id> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(id) = self.try_into() {
            write!(f, "{:?}", id)
        } else {
            f.debug_tuple("RawId")
                .field(&format!("0x{:x}", self.0))
                .finish()
        }
    }
}

impl<Id: fmt::Display + FromPrimitive + Copy> fmt::Display for RawId<Id> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(id) = self.try_into() {
            write!(f, "{}", id)
        } else {
            f.debug_tuple("RawId")
                .field(&format!("0x{:x}", self.0))
                .finish()
        }
    }
}

impl<Id: FromPrimitive + PartialEq + Copy> PartialEq<Id> for RawId<Id> {
    fn eq(&self, other: &Id) -> bool {
        self.try_into().map(|x| x == *other).unwrap_or(false)
    }
}
