use crate::common::*;
use std::fmt;

#[repr(u8)]
#[derive(Copy, Clone, Debug, FromPrimitive, ToPrimitive)]
pub enum OutputReportId {
    RumbleSubcmd = 0x01,
    MCUFwUpdate = 0x03,
    RumbleOnly = 0x10,
}

// Console -> Joy-Con
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct OutputReport {
    pub report_id: OutputReportId,
    pub packet_counter: u8,
    pub rumble_data: RumbleData,
    pub subcmd: SubcommandRequest,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SubcommandRequest {
    pub subcommand_id: SubcommandId,
    pub u: SubcommandRequestData,
}

impl fmt::Debug for SubcommandRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut out = f.debug_struct("SubcommandRequest");
        match self.subcommand_id {
            SubcommandId::SetInputReportMode => {
                out.field("subcommand", unsafe { &self.u.input_report_mode })
            }
            subcmd => out.field("subcommand", &subcmd),
        };
        out.finish()
    }
}

#[repr(C)]
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

#[repr(C)]
#[derive(Copy, Clone)]
pub union SubcommandRequestData {
    pub nothing: (),
    pub input_report_mode: InputReportMode,
    pub player_lights: PlayerLights,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
// TODO: debug
pub struct PlayerLights(u8);

impl PlayerLights {
    #[allow(clippy::identity_op, clippy::too_many_arguments)]
    pub fn new(
        p0: bool,
        p1: bool,
        p2: bool,
        p3: bool,
        f0: bool,
        f1: bool,
        f2: bool,
        f3: bool,
    ) -> PlayerLights {
        PlayerLights(
            (p0 as u8) << 0
                | (p1 as u8) << 1
                | (p2 as u8) << 2
                | (p3 as u8) << 3
                | (f0 as u8) << 4
                | (f1 as u8) << 5
                | (f2 as u8) << 6
                | (f3 as u8) << 7,
        )
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum InputReportMode {
    StandardFull = 0x30,
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
    pub which_controller: u8,
    // Unknown. Seems to be always 02
    _something: u8,
    // Big endian
    pub mac_address: [u8; 6],
    // Unknown. Seems to be always 01
    _somethingelse: u8,
    // bool
    pub use_spi_colors: u8,
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
