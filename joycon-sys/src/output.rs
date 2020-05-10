//! Structs binary compatible with the HID output reports
//!
//! https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/bluetooth_hid_notes.md#output-reports

use crate::common::*;
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
    pub u: SubcommandRequestUnion,
}

impl OutputReport {
    pub fn new() -> OutputReport {
        OutputReport::default()
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self as *const _ as *const u8, std::mem::size_of_val(self))
        }
    }

    #[cfg(test)]
    pub(crate) unsafe fn as_mcu_subcmd(&self) -> &MCUSubcommand {
        &self.u.mcu_subcmd
    }
}

impl Default for OutputReport {
    fn default() -> Self {
        OutputReport {
            report_id: OutputReportId::RumbleAndSubcmd,
            packet_counter: 0,
            rumble_data: RumbleData::default(),
            u: SubcommandRequestUnion {
                subcmd: SubcommandRequest {
                    subcommand_id: SubcommandId::RequestDeviceInfo,
                    u: SubcommandRequestData { nothing: () },
                },
            },
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
            out.field("mcu_subcmd", unsafe { &self.u.mcu_subcmd });
        }
        out.finish()
    }
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub union SubcommandRequestUnion {
    // For OutputReportId::RumbleAndSubcmd
    pub subcmd: SubcommandRequest,
    // For OutputReportId::RequestMCUData
    pub mcu_subcmd: MCUSubcommand,
}

#[repr(packed)]
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
pub union SubcommandRequestData {
    pub nothing: (),
    pub imu_enabled: bool,
    pub input_report_mode: InputReportId,
    pub player_lights: PlayerLights,
    pub mcu_state: MCUState,
    pub mcu_cmd: MCUCmd,
    pub spi_read: SPIReadRequest,
    pub imu_sensitivity: IMUSensitivity,
}

#[repr(packed)]
#[derive(Copy, Clone, Default)]
pub struct IMUSensitivity {
    pub gyro_sens: GyroSens,
    pub acc_sens: AccSens,
    pub gyro_perf_rate: GyroPerfRate,
    pub acc_anti_aliasing: AccAntiAliasing,
}

/// Sensitivity range of the gyroscope.
///
/// If using DPS2000 for example, the gyroscope can measure values of
/// up to +-2000 degree per second for a total range of 4000 DPS over
/// the 16 bit raw value.
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum GyroSens {
    DPS250 = 0,
    DPS500 = 1,
    DPS1000 = 2,
    DPS2000 = 3,
}

impl GyroSens {
    pub fn range_dps(self) -> u16 {
        match self {
            GyroSens::DPS250 => 600,
            GyroSens::DPS500 => 1000,
            GyroSens::DPS1000 => 2000,
            GyroSens::DPS2000 => 4000,
        }
    }
}

impl Default for GyroSens {
    fn default() -> Self {
        GyroSens::DPS2000
    }
}

/// Sensitivity range of the accelerometer.
///
/// If using G4 for example, the accelerometer can measure values of
/// up to +-4G for a total range of 8G over the 16 bit raw value.
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum AccSens {
    G8 = 0,
    G4 = 1,
    G2 = 2,
    G16 = 3,
}

impl AccSens {
    pub fn range_g(self) -> u16 {
        match self {
            AccSens::G8 => 16,
            AccSens::G4 => 8,
            AccSens::G2 => 4,
            AccSens::G16 => 32,
        }
    }
}

impl Default for AccSens {
    fn default() -> Self {
        AccSens::G8
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum GyroPerfRate {
    Hz833 = 0,
    Hz208 = 1,
}

impl Default for GyroPerfRate {
    fn default() -> Self {
        GyroPerfRate::Hz208
    }
}

/// Anti-aliasing setting of the accelerometer.
///
/// Accelerations frequencies above the value are ignored using a low-pass filter.
///
/// See https://blog.endaq.com/filter-selection-for-shock-and-vibration-applications.
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum AccAntiAliasing {
    Hz200 = 0,
    Hz100 = 1,
}

impl Default for AccAntiAliasing {
    fn default() -> Self {
        AccAntiAliasing::Hz100
    }
}

#[repr(packed)]
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
