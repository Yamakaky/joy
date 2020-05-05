use num::{FromPrimitive, ToPrimitive};
use std::fmt;
use std::marker::PhantomData;

pub const NINTENDO_VENDOR_ID: u16 = 1406;

pub const JOYCON_L_BT: u16 = 0x2006;
pub const JOYCON_R_BT: u16 = 0x2007;
pub const PRO_CONTROLLER: u16 = 0x2009;
pub const JOYCON_CHARGING_GRIP: u16 = 0x200e;

// All unused values are a Nop
#[repr(u8)]
#[derive(Copy, Clone, Debug, FromPrimitive, ToPrimitive, PartialEq, Eq)]
pub enum SubcommandId {
    GetOnlyControllerState = 0x00,
    BluetoothManualPairing = 0x01,
    RequestDeviceInfo = 0x02,
    SetInputReportMode = 0x03,
    SetPlayerLights = 0x30,
    EnableIMU = 0x40,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct RawId<Id>(pub u8, PhantomData<Id>);

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
                .field(&format!("{:x}", self.0))
                .finish()
        }
    }
}
