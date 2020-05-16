//! Structs binary compatible with the HID output reports
//!
//! https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/bluetooth_hid_notes.md#output-reports

use crate::common::*;
use crate::light;
use crate::mcu::ir::*;
use crate::mcu::*;
use crate::spi::*;
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
/// It is binary compatible and can be directly casted from the raw HID bytes.
#[repr(packed)]
#[derive(Copy, Clone)]
pub struct OutputReport {
    pub report_id: OutputReportId,
    pub packet_counter: u8,
    pub rumble_data: RumbleData,
    u: OutputReportUnion,
}

impl OutputReport {
    pub fn new() -> OutputReport {
        OutputReport::default()
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
                subcommand_id: SubcommandId::SetMCUConf,
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
            packet_missing: true,
            missed_packet_id: packet_id,
            ack_packet_id: 0,
        })
    }

    pub fn ir_ack(packet_id: u8) -> OutputReport {
        OutputReport::ir_build(IRAckRequestPacket {
            packet_missing: false,
            missed_packet_id: 0,
            ack_packet_id: packet_id,
        })
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self as *const _ as *const u8, std::mem::size_of_val(self))
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
            subcommand_id: SubcommandId::RequestDeviceInfo,
            u: SubcommandRequestUnion { nothing: () },
        }
        .into()
    }
}

impl From<SubcommandRequest> for OutputReport {
    fn from(subcmd: SubcommandRequest) -> Self {
        OutputReport {
            report_id: OutputReportId::RumbleAndSubcmd,
            packet_counter: 0,
            rumble_data: RumbleData::default(),
            u: OutputReportUnion { subcmd },
        }
    }
}

impl From<MCURequest> for OutputReport {
    fn from(mcu_request: MCURequest) -> Self {
        OutputReport {
            report_id: OutputReportId::RequestMCUData,
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
    // For OutputReportId::RumbleAndSubcmd
    subcmd: SubcommandRequest,
    // For OutputReportId::RequestMCUData
    mcu_request: MCURequest,
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct SubcommandRequest {
    pub subcommand_id: SubcommandId,
    pub u: SubcommandRequestUnion,
}

impl From<MCUCommand> for SubcommandRequest {
    fn from(mcu_cmd: MCUCommand) -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::SetMCUConf,
            u: SubcommandRequestUnion { mcu_cmd },
        }
    }
}

impl fmt::Debug for SubcommandRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut out = f.debug_struct("SubcommandRequest");
        match self.subcommand_id {
            SubcommandId::SetInputReportMode => {
                out.field("subcommand", unsafe { &self.u.input_report_mode })
            }
            SubcommandId::SetMCUConf => out.field("subcommand", unsafe { &self.u.mcu_cmd }),
            subcmd => out.field("subcommand", &subcmd),
        };
        out.finish()
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct RumbleData {
    pub raw: [u8; 8],
}

impl Default for RumbleData {
    fn default() -> Self {
        RumbleData {
            raw: [0x00, 0x01, 0x40, 0x40, 0x00, 0x01, 0x40, 0x40],
        }
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub union SubcommandRequestUnion {
    pub nothing: (),
    pub imu_enabled: bool,
    pub input_report_mode: InputReportId,
    player_lights: light::PlayerLights,
    home_light: light::HomeLight,
    pub mcu_mode: MCUMode,
    pub mcu_cmd: MCUCommand,
    pub spi_read: SPIReadRequest,
    pub imu_sensitivity: crate::imu::Sensitivity,
}

impl From<light::PlayerLights> for SubcommandRequest {
    fn from(player_lights: light::PlayerLights) -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::SetPlayerLights,
            u: SubcommandRequestUnion { player_lights },
        }
    }
}

impl From<light::HomeLight> for SubcommandRequest {
    fn from(home_light: light::HomeLight) -> Self {
        SubcommandRequest {
            subcommand_id: SubcommandId::SetHomeLight,
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
