//! Structs binary compatible with the HID input reports
//!
//! https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/bluetooth_hid_notes.md#input-reports

use crate::common::*;
use num::{FromPrimitive, ToPrimitive};
use std::fmt;
use std::marker::PhantomData;

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

#[repr(u8)]
#[derive(Copy, Clone, Debug, FromPrimitive, ToPrimitive, PartialEq, Eq)]
pub enum InputReportId {
    Normal = 0x3F,
    Standard = 0x21,
    MCUFwUpdate = 0x23,
    StandardFull = 0x30,
    StandardFullMCU = 0x31,
    // 0x32 not used
    // 0x33 not used
}

/// Describes a HID report from the JoyCon.
///
/// It is binary compatible and can be directly casted from the raw HID bytes.
///
/// ```
/// let mut buffer = [0u8; size_of::<InputReport>()];
/// read_hid_report(&mut buffer);
/// let report = unsafe { &*(&buffer as *const _ as *const InputReport)}
/// ```
#[repr(packed)]
#[derive(Copy, Clone)]
pub struct InputReport {
    pub report_id: RawId<InputReportId>,
    pub u: InputReportContent,
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub union InputReportContent {
    pub normal: NormalInputReportContent,
    pub standard: StandardInputReportContent,
}
#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct NormalInputReportContent {
    pub buttons: [u8; 2],
    pub stick: u8,
    _filler: [u8; 8],
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct StandardInputReportContent {
    pub timer: u8,
    pub info: DeviceStatus,
    pub buttons: ButtonsStatus,
    pub left_stick: StickStatus,
    pub right_stick: StickStatus,
    pub vibrator: u8,
    pub u: ExtraData,
}

impl fmt::Debug for InputReport {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use InputReportId::*;

        let mut out = f.debug_struct("InputReport");
        out.field("report_id", &self.report_id);
        match self.report_id.try_into() {
            Some(Standard) | Some(StandardFull) | Some(StandardFullMCU) => {
                let content = unsafe { &self.u.standard };
                out.field("timer", &content.timer)
                    .field("info", &content.info)
                    .field("buttons", &content.buttons)
                    .field("left_stick", &content.left_stick)
                    .field("right_stick", &content.right_stick)
                    .field("vibrator", &content.vibrator);
            }
            _ => {}
        }

        match self.report_id.try_into() {
            Some(InputReportId::Normal) => {
                out.field("pote", unsafe { &self.u.normal });
            }
            Some(InputReportId::Standard) => {
                out.field("subcommand_reply", unsafe {
                    &self.u.standard.u.subcmd_reply
                });
            }
            Some(InputReportId::MCUFwUpdate) => {
                out.field("mcu_fw_update", &"[data]");
            }
            // TODO: mask MCU
            Some(InputReportId::StandardFull) => {
                out.field("subcommand_reply", unsafe {
                    &self.u.standard.u.gyro_acc_nfc_ir
                });
            }
            Some(InputReportId::StandardFullMCU) => {
                out.field("subcommand_reply", unsafe {
                    &self.u.standard.u.gyro_acc_nfc_ir
                });
            }
            None => {}
        };
        out.finish()
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DeviceStatus(u8);

impl fmt::Debug for DeviceStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let battery = self.0 >> 4;
        f.debug_struct("DeviceInfo")
            .field(
                "battery",
                &match battery {
                    8 => "full",
                    6 => "medium",
                    4 => "low",
                    2 => "critical",
                    0 => "empty",
                    _ => "<unknown>",
                },
            )
            .field(
                "type",
                &match (self.0 >> 1) & 3 {
                    0 => "Pro Controller",
                    3 => "JoyCon",
                    _ => "<unknown",
                },
            )
            .field("charging", &((self.0 & 1) == 1))
            .finish()
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ButtonsStatus {
    pub right: RightButtons,
    pub middle: MiddleButtons,
    pub left: LeftButtons,
}

bitfield::bitfield! {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct RightButtons(u8);
    impl Debug;
    pub y, _: 0;
    pub x, _: 1;
    pub b, _: 2;
    pub a, _: 3;
    pub sr, _: 4;
    pub sl, _: 5;
    pub r, _: 6;
    pub zr, _: 7;
}
bitfield::bitfield! {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct MiddleButtons(u8);
    impl Debug;
    pub minus, _: 0;
    pub plus, _: 1;
    pub rstick, _: 2;
    pub lstick, _: 3;
    pub home, _: 4;
    pub capture, _: 5;
    pub _unused, _: 6;
    pub charging_grip, _: 7;
}

bitfield::bitfield! {
    #[derive(Copy, Clone)]
    pub struct LeftButtons(u8);
    impl Debug;
    pub down, _: 0;
    pub up, _: 1;
    pub right, _: 2;
    pub left, _: 3;
    pub sr, _: 4;
    pub sl, _: 5;
    pub l, _: 6;
    pub zl, _: 7;
}

pub enum Button {
    N,
    S,
    E,
    W,
    L,
    R,
    ZL,
    ZR,
    L3,
    R3,
    UP,
    DOWN,
    LEFT,
    RIGHT,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct StickStatus {
    pub data: [u8; 3],
}

impl StickStatus {
    pub fn h(self) -> u16 {
        u16::from(self.data[0]) | u16::from(self.data[1] & 0xf) << 8
    }

    pub fn v(self) -> u16 {
        u16::from(self.data[1]) >> 4 | u16::from(self.data[2]) << 4
    }
}

impl fmt::Debug for StickStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("StickStatus")
            .field(&self.h())
            .field(&self.v())
            .finish()
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union ExtraData {
    pub subcmd_reply: SubcommandReply,
    pub mcu_fw_update: [u8; 37],
    pub gyro_acc_nfc_ir: GyroAccNFCIR,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SubcommandReply {
    pub ack: Ack,
    pub subcommand_id: RawId<SubcommandId>,
    pub u: SubcommandReplyData,
}

impl fmt::Debug for SubcommandReply {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut out = f.debug_struct("SubcommandReply");
        out.field("ack", &self.ack);
        match self.subcommand_id.try_into() {
            Some(SubcommandId::RequestDeviceInfo) => {
                out.field("device_info", unsafe { &self.u.device_info })
            }
            Some(subcmd) => out.field("subcommand", &subcmd),
            None => out.field("subcommand_id", &self.subcommand_id),
        };
        out.finish()
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Ack(u8);

impl fmt::Debug for Ack {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 == 0 {
            f.debug_tuple("NAck").finish()
        } else {
            let data = self.0 & 0x7f;
            let mut out = f.debug_tuple("Ack");
            if data != 0 {
                out.field(&data);
            }
            out.finish()
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union SubcommandReplyData {
    pub device_info: DeviceInfo,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct DeviceInfo {
    pub firmware_version: [u8; 2],
    // 1=Left Joy-Con, 2=Right Joy-Con, 3=Pro Controller
    pub which_controller: RawId<WhichController>,
    // Unknown. Seems to be always 02
    _something: u8,
    // Big endian
    pub mac_address: [u8; 6],
    // Unknown. Seems to be always 01
    _somethingelse: u8,
    // bool
    pub use_spi_colors: u8,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, FromPrimitive, ToPrimitive)]
pub enum WhichController {
    LeftJoyCon = 1,
    RightJoyCon = 2,
    ProController = 3,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GyroAccNFCIR {
    pub gyro_acc_frames: [[u8; 12]; 3],
    pub nfc_ir_data: [u8; 313],
}

impl fmt::Debug for GyroAccNFCIR {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("GyroAccNFCIR")
            .field("gyro_acc_frames", &self.gyro_acc_frames)
            .field("nfc_ir_data", &"[data]")
            .finish()
    }
}
