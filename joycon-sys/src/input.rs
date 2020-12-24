//! Structs binary compatible with the HID input reports
//!
//! https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/bluetooth_hid_notes.md#input-reports

use crate::common::*;
use crate::imu;
use crate::mcu::ir::*;
use crate::mcu::*;
use crate::spi::*;
use num::FromPrimitive;
use std::fmt;

/// Describes a HID report from the JoyCon.
///
/// ```ignore
/// let mut report = InputReport::new();
/// read_hid_report(buffer.as_bytes_mut());
/// ```
#[repr(packed)]
#[derive(Copy, Clone)]
pub struct InputReport {
    report_id: RawId<InputReportId>,
    u: InputReportContent,
}

impl InputReport {
    pub fn new() -> InputReport {
        InputReport::default()
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self as *mut _ as *mut u8, std::mem::size_of_val(self))
        }
    }

    pub fn validate(&self) {
        match self.report_id.try_into() {
            Some(_) => {
                if let Some(rep) = self.subcmd_reply() {
                    rep.validate()
                }
                if let Some(rep) = self.mcu_report() {
                    rep.validate()
                }
            }
            None => panic!("unknown report id {:x?}", self.report_id),
        }
    }

    pub fn normal(&self) -> Option<&NormalInputReport> {
        if self.report_id == InputReportId::Normal {
            Some(unsafe { &self.u.normal })
        } else {
            None
        }
    }

    pub fn standard(&self) -> Option<&StandardInputReport> {
        if self.report_id == InputReportId::StandardAndSubcmd
            || self.report_id == InputReportId::StandardFull
            || self.report_id == InputReportId::StandardFullMCU
        {
            Some(unsafe { &self.u.standard })
        } else {
            None
        }
    }

    pub fn subcmd_reply(&self) -> Option<&SubcommandReply> {
        if self.report_id == InputReportId::StandardAndSubcmd {
            Some(unsafe { &self.u.standard.u.subcmd_reply })
        } else {
            None
        }
    }

    pub fn imu_frames(&self) -> Option<&[imu::Frame; 3]> {
        if self.report_id == InputReportId::StandardFull
            || self.report_id == InputReportId::StandardFullMCU
        {
            Some(unsafe { &self.u.standard.u.imu_mcu.imu_frames })
        } else {
            None
        }
    }

    pub fn mcu_report(&self) -> Option<&MCUReport> {
        if self.report_id == InputReportId::StandardFullMCU {
            Some(unsafe { &self.u.standard.u.imu_mcu.mcu_report })
        } else {
            None
        }
    }

    #[cfg(test)]
    pub(crate) unsafe fn u_mcu_report(&self) -> &MCUReport {
        &self.u.standard.u.imu_mcu.mcu_report
    }
}

impl Default for InputReport {
    fn default() -> Self {
        // Whatever value
        InputReport {
            report_id: InputReportId::Normal.into(),
            u: InputReportContent {
                normal: NormalInputReport::default(),
            },
        }
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
union InputReportContent {
    normal: NormalInputReport,
    standard: StandardInputReport,
}

#[repr(packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct NormalInputReport {
    pub buttons: [u8; 2],
    pub stick: u8,
    _filler: [u8; 8],
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct StandardInputReport {
    pub timer: u8,
    pub info: DeviceStatus,
    pub buttons: ButtonsStatus,
    pub left_stick: Stick,
    pub right_stick: Stick,
    pub vibrator: u8,
    u: StandardInputReportUnion,
}

impl fmt::Debug for InputReport {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use InputReportId::*;

        let mut out = f.debug_struct("InputReport");
        out.field("report_id", &self.report_id);
        match self.report_id.try_into() {
            Some(StandardAndSubcmd) | Some(StandardFull) | Some(StandardFullMCU) => {
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
            Some(InputReportId::StandardAndSubcmd) => {
                out.field("subcommand_reply", unsafe {
                    &self.u.standard.u.subcmd_reply
                });
            }
            Some(InputReportId::MCUFwUpdate) => {
                out.field("mcu_fw_update", &"[data]");
            }
            // TODO: mask MCU
            Some(InputReportId::StandardFull) => {
                out.field("subcommand_reply", unsafe { &self.u.standard.u.imu_mcu });
            }
            Some(InputReportId::StandardFullMCU) => {
                out.field("subcommand_reply", unsafe { &self.u.standard.u.imu_mcu });
            }
            None => {}
        };
        out.finish()
    }
}

bitfield::bitfield! {
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    pub struct DeviceStatus(u8);
    impl Debug;

    pub connected, _: 0;
    pub u8, into DeviceType, device_type, _: 2, 1;
    pub charging, _: 4;
    pub u8, into BatteryLevel, battery_level, _: 7, 5;
}

#[derive(Debug, Copy, Clone, FromPrimitive)]
pub enum DeviceType {
    ProController = 0,
    Joycon = 3,
}

impl From<u8> for DeviceType {
    fn from(v: u8) -> Self {
        DeviceType::from_u8(v).expect("unexpected device type")
    }
}

#[derive(Debug, Copy, Clone, FromPrimitive, Eq, PartialEq, Ord, PartialOrd)]
pub enum BatteryLevel {
    Empty = 0,
    Critical = 1,
    Low = 2,
    Medium = 3,
    Full = 4,
}

impl From<u8> for BatteryLevel {
    fn from(v: u8) -> Self {
        BatteryLevel::from_u8(v).expect("unexpected battery level")
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct ButtonsStatus {
    pub right: RightButtons,
    pub middle: MiddleButtons,
    pub left: LeftButtons,
}

impl fmt::Display for ButtonsStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.right.a() {
            write!(f, " A")?;
        }
        if self.right.b() {
            write!(f, " B")?;
        }
        if self.right.x() {
            write!(f, " X")?;
        }
        if self.right.y() {
            write!(f, " Y")?;
        }
        if self.left.up() {
            write!(f, " UP")?;
        }
        if self.left.down() {
            write!(f, " DOWN")?;
        }
        if self.left.left() {
            write!(f, " LEFT")?;
        }
        if self.left.right() {
            write!(f, " RIGHT")?;
        }
        if self.left.l() {
            write!(f, " L")?;
        }
        if self.left.zl() {
            write!(f, " ZL")?;
        }
        if self.right.r() {
            write!(f, " R")?;
        }
        if self.right.zr() {
            write!(f, " ZR")?;
        }
        if self.left.sl() || self.right.sl() {
            write!(f, " SR")?;
        }
        if self.left.sr() || self.right.sr() {
            write!(f, " SR")?;
        }
        if self.middle.lstick() {
            write!(f, " L3")?;
        }
        if self.middle.rstick() {
            write!(f, " R3")?;
        }
        if self.middle.minus() {
            write!(f, " -")?;
        }
        if self.middle.plus() {
            write!(f, " +")?;
        }
        if self.middle.capture() {
            write!(f, " CAPTURE")?;
        }
        if self.middle.home() {
            write!(f, " HOME")?;
        }
        Ok(())
    }
}

bitfield::bitfield! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Default)]
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
    #[repr(transparent)]
    #[derive(Copy, Clone, Default)]
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
    #[repr(transparent)]
    #[derive(Copy, Clone, Default)]
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

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct Stick {
    data: [u8; 3],
}

impl Stick {
    pub fn x(self) -> u16 {
        u16::from(self.data[0]) | u16::from(self.data[1] & 0xf) << 8
    }

    pub fn y(self) -> u16 {
        u16::from(self.data[1]) >> 4 | u16::from(self.data[2]) << 4
    }
}

impl fmt::Debug for Stick {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Stick")
            .field(&self.x())
            .field(&self.y())
            .finish()
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
union StandardInputReportUnion {
    _nothing: (),
    subcmd_reply: SubcommandReply,
    _mcu_fw_update: [u8; 37],
    imu_mcu: IMUMCU,
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct SubcommandReply {
    pub ack: Ack,
    subcommand_id: RawId<SubcommandId>,
    u: SubcommandReplyUnion,
}

impl SubcommandReply {
    pub fn validate(&self) {
        assert!(
            self.subcommand_id.try_into().is_some(),
            "invalid subcmd id{:?}",
            self.subcommand_id
        )
    }

    pub fn id(&self) -> Option<SubcommandId> {
        self.subcommand_id.try_into()
    }

    pub fn device_info(&self) -> Option<&DeviceInfo> {
        if self.subcommand_id == SubcommandId::RequestDeviceInfo {
            Some(unsafe { &self.u.device_info })
        } else {
            None
        }
    }

    pub fn spi_result(&self) -> Option<&SPIReadResult> {
        if self.subcommand_id == SubcommandId::SPIRead {
            Some(unsafe { &self.u.spi_read })
        } else {
            None
        }
    }

    pub fn spi_write_success(&self) -> Option<bool> {
        if self.subcommand_id == SubcommandId::SPIWrite {
            Some(self.ack.is_ok() && unsafe { self.u.spi_write.success() })
        } else {
            None
        }
    }

    pub unsafe fn ir_status(&self) -> (RawId<MCUReportId>, IRStatus) {
        // seems to be true
        assert_eq!(self.subcommand_id, SubcommandId::SetMCUConf);
        self.u.ir_status
    }
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

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct Ack(u8);

impl Ack {
    pub fn is_ok(self) -> bool {
        (self.0 & 0x80) != 0
    }
}

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

#[repr(packed)]
#[derive(Copy, Clone)]
union SubcommandReplyUnion {
    // add to validate() when adding variant
    device_info: DeviceInfo,
    spi_read: SPIReadResult,
    spi_write: SPIWriteResult,
    ir_status: (RawId<MCUReportId>, IRStatus),
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct DeviceInfo {
    pub firmware_version: FirmwareVersion,
    // 1=Left Joy-Con, 2=Right Joy-Con, 3=Pro Controller
    pub which_controller: RawId<WhichController>,
    // Unknown. Seems to be always 02
    _something: u8,
    // Big endian
    pub mac_address: MACAddress,
    // Unknown. Seems to be always 01
    _somethingelse: u8,
    // bool
    pub use_spi_colors: RawId<UseSPIColors>,
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct FirmwareVersion(pub [u8; 2]);

impl fmt::Display for FirmwareVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.0[0], self.0[1])
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct MACAddress(pub [u8; 6]);

impl fmt::Display for MACAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, FromPrimitive, ToPrimitive, Eq, PartialEq)]
pub enum WhichController {
    LeftJoyCon = 1,
    RightJoyCon = 2,
    ProController = 3,
}

impl fmt::Display for WhichController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                WhichController::LeftJoyCon => "JoyCon (L)",
                WhichController::RightJoyCon => "JoyCon (R)",
                WhichController::ProController => "Pro Controller",
            }
        )
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, FromPrimitive, ToPrimitive, Eq, PartialEq)]
pub enum UseSPIColors {
    No = 0,
    WithoutGrip = 1,
    IncludingGrip = 2,
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct IMUMCU {
    imu_frames: [imu::Frame; 3],
    mcu_report: MCUReport,
}

#[cfg(test)]
#[test]
fn check_layout() {
    unsafe {
        let report = InputReport::new();
        assert_eq!(6, offset_of(&report, &report.u.standard.left_stick));
        assert_eq!(
            13,
            offset_of(&report, &report.u.standard.u.imu_mcu.imu_frames)
        );
        assert_eq!(13, offset_of(&report, &report.u.standard.u.subcmd_reply));
        assert_eq!(15, offset_of(&report, &report.u.standard.u.subcmd_reply.u));
        assert_eq!(
            49,
            offset_of(&report, &report.u.standard.u.imu_mcu.mcu_report)
        );
        assert_eq!(362, std::mem::size_of_val(&report));
        assert!(37 >= std::mem::size_of::<SubcommandReply>());
    }
}
