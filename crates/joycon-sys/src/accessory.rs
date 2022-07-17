use std::fmt;

use crate::{RawId, U16LE};

// subcommand id 0x58
//
// Maybe arg 2 is a device selector? Check with pokeball plus
//
// arg [4,0,0,2], ret [0,8,0,0,0,0,0,44]
// arg [4,4,5,2], ret [0,8,0,0,0,0,200]
// arg [4,4,50,2], ret [0,8,0,0,0,0,5,0,0,14]
// arg [4,4,10,2], ret [0,20,0,0,0,0,244,22,0,0,230,5,0,0,243,11,0,0,234,12, 0, 0]
// get ringcon calibration: arg [4,4,26,2]
//                          ret [0,20,0,0,0,0] + [135, 8, 28, 0, 48, 247, 243, 0, 44, 12, 224]
// write ringcon calibration: arg [20,4,26,1,16] + [135, 8, 28, 0, 48, 247, 243, 0, 44, 12, 224]
//                            ret [0, 4]
// get number steps offline ringcon: arg [4,4,49,2], ret [0,8,0,0,0,0,nb_steps, 0,0, 127|143]
// reset number steps offline ringcon: arg [8,4,49,1,4], ret [0,4]

#[repr(u8)]
#[derive(Copy, Clone, Debug, FromPrimitive, ToPrimitive, PartialEq, Eq)]
pub enum AccessoryCommandId {
    Get = 4,
    Reset = 8,
    Write = 20,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, FromPrimitive, ToPrimitive, PartialEq, Eq)]
pub enum AccessoryType {
    Ringcon = 4,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, FromPrimitive, ToPrimitive, PartialEq, Eq)]
pub enum RingconItemId {
    Calibration = 26,
    OfflineSteps = 49,
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub struct AccessoryCommand {
    id: RawId<AccessoryCommandId>,
    ty: RawId<AccessoryType>,
    item: RawId<RingconItemId>,
    maybe_includes_arg: u8,
    maybe_arg_size: u8,
    raw: [u8; 18],
}
impl AccessoryCommand {
    pub fn get_offline_steps() -> Self {
        AccessoryCommand {
            id: AccessoryCommandId::Get.into(),
            ty: AccessoryType::Ringcon.into(),
            item: RingconItemId::OfflineSteps.into(),
            maybe_includes_arg: 2,
            maybe_arg_size: 0,
            raw: [0; 18],
        }
    }

    // Known CRC values:
    //    0 0
    //    1 127
    //    2 254
    //    3 129
    //    4 113
    //    5 14
    //    0x64 74
    //    0xf0 173
    //    0x100 200
    //    0x101 183
    //    0x103 73
    //    0x104 185
    //    0x1f4 20
    pub fn write_offline_steps(steps: u16, sum: u8) -> Self {
        let steps = steps.to_le_bytes();
        AccessoryCommand {
            id: AccessoryCommandId::Reset.into(),
            ty: AccessoryType::Ringcon.into(),
            item: RingconItemId::OfflineSteps.into(),
            maybe_includes_arg: 1,
            maybe_arg_size: 4,
            raw: [
                steps[0], steps[1], 0, sum, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        }
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct AccessoryResponse {
    //254: nothing connected
    error: u8,
    len: u8,
    unknown_0x00: [u8; 4],
    u: AccessoryResponseUnion,
}

impl AccessoryResponse {
    fn check_error(&self) -> Result<(), Error> {
        match self.error {
            0 => Ok(()),
            254 => Err(Error::NoAccessoryConnected),
            e => Err(Error::Other(e)),
        }
    }

    pub fn offline_steps(&self) -> Result<OfflineSteps, Error> {
        self.check_error()?;
        Ok(unsafe { self.u.offline_steps })
    }
}

impl fmt::Debug for AccessoryResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccessoryResponse")
            .field("maybe_error", &self.error)
            .field("always_0x00", &self.unknown_0x00)
            .field("data", unsafe { &&self.u.raw[..self.len as usize] })
            .finish()
    }
}

#[derive(Copy, Clone)]
union AccessoryResponseUnion {
    offline_steps: OfflineSteps,
    raw: [u8; 20],
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub struct OfflineSteps {
    pub steps: U16LE,
    unknown0x00: u8,
    maybe_crc: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum Error {
    NoAccessoryConnected,
    Other(u8),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::NoAccessoryConnected => f.write_str("no accessory connected"),
            Error::Other(e) => f.write_fmt(format_args!("unknown accessory error: {}", e)),
        }
    }
}
