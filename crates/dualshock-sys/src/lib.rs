#[macro_use]
extern crate num_derive;

use std::{fmt, marker::PhantomData};

use num::{FromPrimitive, ToPrimitive};

pub mod input;
pub mod output;

pub const HID_VENDOR_ID: u16 = 0x54c;
pub const HID_PRODUCT_ID_NEW: u16 = 0x9cc;
pub const HID_PRODUCT_ID_OLD: u16 = 0x5c4;

pub const DS4_REPORT_RATE: u32 = 250;
pub const DS4_REPORT_DT: f64 = 1. / DS4_REPORT_RATE as f64;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ConnectionType {
    Bluetooth,
    USB,
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

#[derive(Copy, Clone, Default, Eq, PartialEq)]
pub struct I16LE(pub [u8; 2]);

impl From<i16> for I16LE {
    fn from(u: i16) -> I16LE {
        I16LE(u.to_le_bytes())
    }
}

impl From<I16LE> for i16 {
    fn from(u: I16LE) -> i16 {
        i16::from_le_bytes(u.0)
    }
}

impl fmt::Debug for I16LE {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        i16::from(*self).fmt(f)
    }
}
