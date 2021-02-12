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

    pub fn write_offline_steps() -> Self {
        AccessoryCommand {
            id: AccessoryCommandId::Reset.into(),
            ty: AccessoryType::Ringcon.into(),
            item: RingconItemId::OfflineSteps.into(),
            maybe_includes_arg: 1,
            maybe_arg_size: 4,
            raw: [2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        }
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct AccessoryResponse {
    maybe_error: u8,
    len: u8,
    unknown_0x00: [u8; 4],
    u: AccessoryResponseUnion,
}

impl AccessoryResponse {
    pub fn offline_steps(&self) -> &OfflineSteps {
        unsafe { &self.u.offline_steps }
    }
}

impl fmt::Debug for AccessoryResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccessoryResponse")
            .field("maybe_error", &self.maybe_error)
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
pub struct OfflineSteps {
    steps: U16LE,
    unknown0x00: u8,
    maybe_crc: u8,
}
