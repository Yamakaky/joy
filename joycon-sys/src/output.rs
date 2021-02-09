//! Structs binary compatible with the HID output reports
//!
//! https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/bluetooth_hid_notes.md#output-reports

use crate::light;
use crate::mcu::ir::*;
use crate::mcu::*;
use crate::spi::*;
use crate::{common::*, imu::IMUMode};
use std::fmt;

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, FromPrimitive, ToPrimitive)]
pub enum OutputReportId {
    RumbleAndSubcmd = 0x01,
    MCUFwUpdate = 0x03,
    RumbleOnly = 0x10,
    RequestMCUData = 0x11,
}

/// Describes a HID report sent to the JoyCon.
///
/// ```ignore
/// let report = OutputReport::from(SubcommandRequest::request_device_info());
/// write_hid_report(report.as_bytes());
/// ```
#[repr(packed)]
#[derive(Copy, Clone)]
pub struct OutputReport {
    pub report_id: RawId<OutputReportId>,
    pub packet_counter: u8,
    pub rumble_data: RumbleData,
    u: OutputReportUnion,
}

impl OutputReport {
    pub fn new() -> OutputReport {
        unsafe { std::mem::zeroed() }
    }

    pub fn set_registers(regs: &[ir::Register]) -> (OutputReport, &[ir::Register]) {
        let size = regs.len().min(9);
        let mut regs_fixed = [ir::Register::default(); 9];
        regs_fixed[..size].copy_from_slice(&regs[..size]);
        let mcu_cmd = MCUCommand::set_ir_registers(MCURegisters {
            len: size as u8,
            regs: regs_fixed,
        });
        (
            SubcommandRequest {
                subcommand_id: SubcommandId::SetMCUConf.into(),
                u: SubcommandRequestUnion { mcu_cmd },
            }
            .into(),
            &regs[size..],
        )
    }

    fn ir_build(ack_request_packet: IRAckRequestPacket) -> OutputReport {
        let mcu_request = MCURequest::from(IRRequest::from(ack_request_packet));
        mcu_request.into()
    }

    pub fn ir_resend(packet_id: u8) -> OutputReport {
        OutputReport::ir_build(IRAckRequestPacket {
            packet_missing: Bool::True.into(),
            missed_packet_id: packet_id,
            ack_packet_id: 0,
        })
    }

    pub fn ir_ack(packet_id: u8) -> OutputReport {
        OutputReport::ir_build(IRAckRequestPacket {
            packet_missing: Bool::False.into(),
            missed_packet_id: 0,
            ack_packet_id: packet_id,
        })
    }

    pub fn set_rumble(rumble: RumbleData) -> OutputReport {
        OutputReport {
            report_id: OutputReportId::RumbleOnly.into(),
            packet_counter: 0,
            rumble_data: rumble,
            u: OutputReportUnion { nothing: () },
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self as *const _ as *const u8, std::mem::size_of_val(self))
        }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self as *mut _ as *mut u8, std::mem::size_of_val(self))
        }
    }

    pub fn subcmd_request(&self) -> Option<&SubcommandRequest> {
        if self.report_id == OutputReportId::RumbleAndSubcmd {
            Some(unsafe { &self.u.subcmd })
        } else {
            None
        }
    }

    #[cfg(test)]
    pub(crate) unsafe fn as_mcu_request(&self) -> &MCURequest {
        &self.u.mcu_request
    }

    #[cfg(test)]
    pub(crate) unsafe fn as_mcu_cmd(&self) -> &MCUCommand {
        &self.u.subcmd.u.mcu_cmd
    }
}

impl Default for OutputReport {
    fn default() -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::RequestDeviceInfo.into(),
            u: SubcommandRequestUnion { nothing: () },
        }
        .into()
    }
}

impl From<SubcommandRequest> for OutputReport {
    fn from(subcmd: SubcommandRequest) -> Self {
        OutputReport {
            report_id: OutputReportId::RumbleAndSubcmd.into(),
            packet_counter: 0,
            rumble_data: RumbleData::default(),
            u: OutputReportUnion { subcmd },
        }
    }
}

impl From<MCURequest> for OutputReport {
    fn from(mcu_request: MCURequest) -> Self {
        OutputReport {
            report_id: OutputReportId::RequestMCUData.into(),
            packet_counter: 0,
            rumble_data: RumbleData::default(),
            u: OutputReportUnion { mcu_request },
        }
    }
}

impl fmt::Debug for OutputReport {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut out = f.debug_struct("OutputReport");
        out.field("id", &self.report_id)
            .field("counter", &self.packet_counter);
        if self.report_id == OutputReportId::RumbleAndSubcmd {
            out.field("subcmd", unsafe { &self.u.subcmd });
        } else if self.report_id == OutputReportId::RequestMCUData {
            out.field("mcu_subcmd", unsafe { &self.u.mcu_request });
        }
        out.finish()
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
union OutputReportUnion {
    // For OutputReportId::RumbleOnly
    nothing: (),
    // For OutputReportId::RumbleAndSubcmd
    subcmd: SubcommandRequest,
    // For OutputReportId::RequestMCUData
    mcu_request: MCURequest,
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct SubcommandRequest {
    subcommand_id: RawId<SubcommandId>,
    u: SubcommandRequestUnion,
}

impl SubcommandRequest {
    pub fn id(&self) -> SubcommandId {
        self.subcommand_id.try_into().unwrap()
    }
    pub fn set_imu_mode(imu_mode: IMUMode) -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::SetIMUMode.into(),
            u: SubcommandRequestUnion {
                imu_mode: imu_mode.into(),
            },
        }
    }

    pub fn disable_shipment_mode() -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::SetShipmentMode.into(),
            u: SubcommandRequestUnion {
                shipment_mode: Bool::False.into(),
            },
        }
    }

    pub fn set_input_report_mode(input_report_mode: InputReportId) -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::SetInputReportMode.into(),
            u: SubcommandRequestUnion {
                input_report_mode: input_report_mode.into(),
            },
        }
    }

    pub fn request_device_info() -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::RequestDeviceInfo.into(),
            u: SubcommandRequestUnion { nothing: () },
        }
    }

    pub fn set_mcu_mode(mcu_mode: MCUMode) -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::SetMCUState.into(),
            u: SubcommandRequestUnion {
                mcu_mode: mcu_mode.into(),
            },
        }
    }

    pub fn subcmd_0x59() -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::Unknown0x59.into(),
            u: SubcommandRequestUnion { raw: [0; 38] },
        }
    }

    pub fn subcmd_0x5a() -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::Unknown0x5a.into(),
            u: SubcommandRequestUnion {
                raw: [
                    4, 1, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ],
            },
        }
    }

    pub fn subcmd_0x5b() -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::Unknown0x5b.into(),
            u: SubcommandRequestUnion { raw: [0; 38] },
        }
    }

    pub fn subcmd_0x5c_0() -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::Unknown0x5c.into(),
            u: SubcommandRequestUnion {
                raw: [
                    0, 0, 150, 227, 28, 0, 0, 0, 236, 153, 172, 227, 28, 0, 0, 0, 243, 130, 241,
                    89, 46, 89, 0, 0, 224, 88, 179, 227, 28, 0, 0, 0, 0, 242, 5, 42, 1, 0,
                ],
            },
        }
    }

    pub fn subcmd_0x5c_6() -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::Unknown0x5c.into(),
            u: SubcommandRequestUnion {
                raw: [
                    6, 3, 37, 6, 0, 0, 0, 0, 236, 153, 172, 227, 28, 0, 0, 0, 105, 155, 22, 246,
                    93, 86, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 144, 40, 161, 227, 28, 0,
                ],
            },
        }
    }
}

impl From<MCUCommand> for SubcommandRequest {
    fn from(mcu_cmd: MCUCommand) -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::SetMCUConf.into(),
            u: SubcommandRequestUnion { mcu_cmd },
        }
    }
}

impl fmt::Debug for SubcommandRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut out = f.debug_struct("SubcommandRequest");
        match self.subcommand_id.try_into() {
            Some(SubcommandId::SetInputReportMode) => {
                out.field("report_mode", unsafe { &self.u.input_report_mode })
            }
            Some(SubcommandId::SetMCUConf) => out.field("subcommand", unsafe { &self.u.mcu_cmd }),
            Some(SubcommandId::SPIRead) => out.field("subcommand", unsafe { &self.u.spi_read }),
            Some(SubcommandId::SetIMUMode) => {
                out.field("set_imu_mode", unsafe { &self.u.imu_mode })
            }
            Some(SubcommandId::SetMCUState) => out.field("mcu_state", unsafe { &self.u.mcu_mode }),
            Some(SubcommandId::SetShipmentMode) => {
                out.field("set_shipment_mode", unsafe { &self.u.shipment_mode })
            }
            Some(SubcommandId::EnableVibration) => {
                out.field("enable_vibration", unsafe { &self.u.enable_vibration })
            }
            Some(SubcommandId::SetPlayerLights) => {
                out.field("set_player_lights", unsafe { &self.u.player_lights })
            }
            subcmd @ Some(SubcommandId::GetOnlyControllerState)
            | subcmd @ Some(SubcommandId::GetTriggerButtonsElapsedTime) => {
                out.field("subcommand", &subcmd.expect("unreachable"))
            }
            Some(subcmd) => {
                out.field("subcommand", &subcmd);
                out.field("raw", unsafe { &&self.u.raw })
            }
            None => {
                out.field("subcommand_id", &self.subcommand_id);
                out.field("raw", unsafe { &&self.u.raw })
            }
        };
        out.finish()
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct RumbleData {
    pub left: RumbleSide,
    pub right: RumbleSide,
}

#[repr(packed)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[allow(non_snake_case)]
pub struct RumbleSide {
    hb_freq_msB: u8,
    hb_freq_lsb_amp_high: u8,
    lb_freq_amp_low_msb: u8,
    amp_low_lsB: u8,
}

impl RumbleSide {
    pub fn from_freq(
        mut hi_freq: f32,
        mut hi_amp: f32,
        mut low_freq: f32,
        mut low_amp: f32,
    ) -> RumbleSide {
        hi_freq = hi_freq.max(82.).min(1253.);
        low_freq = low_freq.max(41.).min(626.);
        low_amp = low_amp.max(0.).min(1.);
        hi_amp = hi_amp.max(0.).min(1.);

        let hi_freq_hex = (Self::encode_freq(hi_freq) - 0x60) * 4;
        let low_freq_hex = (Self::encode_freq(low_freq) - 0x40) as u8;
        let hi_amp_hex = ((100. * hi_amp) as u8) << 1;
        let low_amp_hex = ((228. - 128.) * low_amp) as u8 + 0x80;
        RumbleSide::from_encoded(
            [hi_freq_hex as u8, (hi_freq_hex >> 8) as u8],
            hi_amp_hex,
            low_freq_hex,
            [(low_amp_hex & 1) << 7, low_amp_hex >> 1],
        )
    }

    fn encode_freq(f: f32) -> u16 {
        ((f / 10.).log2() * 32.).round() as u16
    }

    fn from_encoded(
        high_freq: [u8; 2],
        high_amp: u8,
        low_freq: u8,
        low_amp: [u8; 2],
    ) -> RumbleSide {
        assert_eq!(high_freq[0] & 0b11, 0);
        assert_eq!(high_freq[1] & 0xfe, 0);
        assert_eq!(high_amp & 1, 0);
        assert!(high_amp <= 0xc8);
        assert_eq!(low_freq & 0x80, 0);
        assert_eq!(low_amp[0] & 0x7f, 0);
        assert!(0x40 <= low_amp[1] && low_amp[1] <= 0x72);
        RumbleSide {
            hb_freq_msB: high_freq[0],
            hb_freq_lsb_amp_high: high_freq[1] | high_amp,
            lb_freq_amp_low_msb: low_freq | low_amp[0],
            amp_low_lsB: low_amp[1],
        }
    }
}

#[test]
fn encode_rumble() {
    let rumble = RumbleSide::from_freq(320., 0., 160., 0.);
    assert_eq!(
        rumble,
        RumbleSide {
            hb_freq_msB: 0x00,
            hb_freq_lsb_amp_high: 0x01,
            lb_freq_amp_low_msb: 0x40,
            amp_low_lsB: 0x40,
        }
    );
}

impl Default for RumbleSide {
    fn default() -> Self {
        RumbleSide::from_freq(320., 0., 160., 0.)
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
union SubcommandRequestUnion {
    nothing: (),
    // TODO: can be 0x2 too
    imu_mode: RawId<IMUMode>,
    input_report_mode: RawId<InputReportId>,
    player_lights: light::PlayerLights,
    home_light: light::HomeLight,
    mcu_mode: RawId<MCUMode>,
    mcu_cmd: MCUCommand,
    spi_read: SPIReadRequest,
    spi_write: SPIWriteRequest,
    imu_sensitivity: crate::imu::Sensitivity,
    shipment_mode: RawId<Bool>,
    enable_vibration: RawId<Bool>,
    raw: [u8; 38],
}

impl From<crate::imu::Sensitivity> for SubcommandRequest {
    fn from(imu_sensitivity: crate::imu::Sensitivity) -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::SetIMUSens.into(),
            u: SubcommandRequestUnion { imu_sensitivity },
        }
    }
}

impl From<SPIReadRequest> for SubcommandRequest {
    fn from(spi_read: SPIReadRequest) -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::SPIRead.into(),
            u: SubcommandRequestUnion { spi_read },
        }
    }
}

impl From<SPIWriteRequest> for SubcommandRequest {
    fn from(spi_write: SPIWriteRequest) -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::SPIWrite.into(),
            u: SubcommandRequestUnion { spi_write },
        }
    }
}

impl From<light::PlayerLights> for SubcommandRequest {
    fn from(player_lights: light::PlayerLights) -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::SetPlayerLights.into(),
            u: SubcommandRequestUnion { player_lights },
        }
    }
}

impl From<light::HomeLight> for SubcommandRequest {
    fn from(home_light: light::HomeLight) -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::SetHomeLight.into(),
            u: SubcommandRequestUnion { home_light },
        }
    }
}

#[cfg(test)]
#[test]
pub fn check_layout() {
    unsafe {
        let report = OutputReport::new();
        assert_eq!(2, offset_of(&report, &report.rumble_data));
        assert_eq!(10, offset_of(&report, &report.u.subcmd.subcommand_id));
        assert_eq!(11, offset_of(&report, &report.u.subcmd.u.mcu_cmd));
        assert_eq!(49, std::mem::size_of_val(&report));
    }
}
