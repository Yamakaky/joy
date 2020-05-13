//! Structs binary compatible with the HID input reports
//!
//! https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/bluetooth_hid_notes.md#input-reports

use crate::common::*;
use crate::imu;
use crate::mcu::ir::*;
use crate::mcu::*;
use crate::spi::*;
use std::fmt;

/// Describes a HID report from the JoyCon.
///
/// It is binary compatible and can be directly casted from the raw HID bytes.
///
/// ```ignore
/// let mut buffer = [0u8; size_of::<InputReport>()];
/// read_hid_report(&mut buffer);
/// let report = unsafe { &*(&buffer as *const _ as *const InputReport)}
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

    pub fn normal(&self) -> Option<&NormalInputReportContent> {
        if self.report_id == InputReportId::Normal {
            Some(unsafe { &self.u.normal })
        } else {
            None
        }
    }

    pub fn standard(&self) -> Option<&StandardInputReportContent> {
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
                normal: NormalInputReportContent::default(),
            },
        }
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
union InputReportContent {
    normal: NormalInputReportContent,
    standard: StandardInputReportContent,
}

#[repr(packed)]
#[derive(Copy, Clone, Debug, Default)]
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
    u: ExtraData,
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

#[repr(packed)]
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

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct ButtonsStatus {
    pub right: RightButtons,
    pub middle: MiddleButtons,
    pub left: LeftButtons,
}

bitfield::bitfield! {
    #[repr(packed)]
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
    #[repr(packed)]
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

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct StickStatus {
    data: [u8; 3],
}

impl StickStatus {
    pub fn x(self) -> u16 {
        u16::from(self.data[0]) | u16::from(self.data[1] & 0xf) << 8
    }

    pub fn y(self) -> u16 {
        u16::from(self.data[1]) >> 4 | u16::from(self.data[2]) << 4
    }
}

impl fmt::Debug for StickStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("StickStatus")
            .field(&self.x())
            .field(&self.y())
            .finish()
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
union ExtraData {
    subcmd_reply: SubcommandReply,
    _mcu_fw_update: [u8; 37],
    imu_mcu: IMUMCU,
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct SubcommandReply {
    pub ack: Ack,
    subcommand_id: RawId<SubcommandId>,
    u: SubcommandReplyData,
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
union SubcommandReplyData {
    // add to validate() when adding variant
    device_info: DeviceInfo,
    spi_read: SPIReadResult,
    ir_status: (RawId<MCUReportId>, IRStatus),
}

#[repr(packed)]
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

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct IMUMCU {
    imu_frames: [imu::Frame; 3],
    mcu_report: MCUReport,
}

impl fmt::Debug for IMUMCU {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("IMUMCU")
            .field("imu_frames", &self.imu_frames)
            .field("mcu_report", &self.mcu_report)
            .finish()
    }
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
