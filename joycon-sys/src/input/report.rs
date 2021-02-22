//! Structs binary compatible with the HID input reports
//!
//! https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/bluetooth_hid_notes.md#input-reports

use crate::{accessory::AccessoryResponse, common::*, imu, input::*, mcu::*, raw_enum, spi::*};
use std::{fmt, mem::size_of_val};

raw_enum! {
    #[id: InputReportId]
    #[union: InputReportUnion]
    #[struct: InputReport]
    pub enum InputReportEnum {
        normal normal_mut: Normal = NormalInputReport,
        standard_subcmd standard_subcmd_mut: StandardAndSubcmd = (
            StandardInputReport,
            SubcommandReply,
        ),
        mcu_fw_update mcu_fw_update_mut: MCUFwUpdate = (),
        standard_full standard_full_mut: StandardFull = (
            StandardInputReport,
            [imu::Frame; 3]
        ),
        standard_full_mcu standard_full_mcu_mut: StandardFullMCU = (
            StandardInputReport,
            [imu::Frame; 3],
            MCUReport
        )
    }
}

// Describes a HID report from the JoyCon.
//
// ```ignore
// let mut report = InputReport::new();
// read_hid_report(buffer.as_bytes_mut());
// ```
//pub struct InputReport {

impl InputReport {
    pub fn is_special(&self) -> bool {
        self.id != InputReportId::Normal
            && self.id != InputReportId::StandardFull
            && self
                .mcu_report()
                .and_then(MCUReport::ir_data)
                .map(|_| false)
                .unwrap_or(true)
    }

    pub fn len(&self) -> usize {
        match self.id.try_into() {
            Some(InputReportId::Normal) => 12,
            Some(InputReportId::StandardAndSubcmd) | Some(InputReportId::StandardFull) => 49,
            Some(InputReportId::StandardFullMCU) => 362,
            Some(InputReportId::MCUFwUpdate) => unimplemented!(),
            None => size_of_val(self),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self as *const _ as *const u8, self.len()) }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self as *mut _ as *mut u8, size_of_val(self)) }
    }

    pub fn validate(&self) {
        match self.id.try_into() {
            Some(_) => {
                if let Some(rep) = self.subcmd_reply() {
                    rep.validate()
                }
                if let Some(rep) = self.mcu_report() {
                    rep.validate()
                }
            }
            None => panic!("unknown report id {:x?}", self.id),
        }
    }

    pub fn standard(&self) -> Option<&StandardInputReport> {
        if self.id == InputReportId::StandardAndSubcmd
            || self.id == InputReportId::StandardFull
            || self.id == InputReportId::StandardFullMCU
        {
            Some(unsafe { &self.u.standard_full.0 })
        } else {
            None
        }
    }

    pub fn subcmd_reply(&self) -> Option<&SubcommandReply> {
        self.standard_subcmd().map(|x| &x.1)
    }

    pub fn imu_frames(&self) -> Option<&[imu::Frame; 3]> {
        if self.id == InputReportId::StandardFull || self.id == InputReportId::StandardFullMCU {
            Some(unsafe { &self.u.standard_full.1 })
        } else {
            None
        }
    }

    pub fn mcu_report(&self) -> Option<&MCUReport> {
        self.standard_full_mcu().map(|x| &x.2)
    }

    #[cfg(test)]
    pub(crate) unsafe fn u_mcu_report(&self) -> &MCUReport {
        &self.u.standard_full_mcu.2
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct NormalInputReport {
    pub buttons: [u8; 2],
    pub stick: u8,
    _filler: [u8; 8],
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct StandardInputReport {
    pub timer: u8,
    pub info: DeviceStatus,
    pub buttons: ButtonsStatus,
    pub left_stick: Stick,
    pub right_stick: Stick,
    pub vibrator: u8,
}

raw_enum! {
    #[pre_id ack ack_mut: Ack]
    #[id: SubcommandId]
    #[union: SubcommandReplyUnion]
    #[struct: SubcommandReply]
    #[raw [u8; 39]]
    pub enum SubcommandReplyEnum {
        controller_state controller_state_mut: GetOnlyControllerState = (),
        bluetooth_manual_pairing bluetooth_manual_pairing_mut: BluetoothManualPairing = (),
        device_info device_info_mut: RequestDeviceInfo = DeviceInfo,
        input_report_mode_result input_report_mode_result_mut: SetInputReportMode = (),
        trigger_buttons_elapsed_time trigger_buttons_elapsed_time_mut: GetTriggerButtonsElapsedTime = [U16LE; 7],
        shipment_mode_result shipment_mode_result_mut: SetShipmentMode = (),
        spi_read_result spi_read_result_mut: SPIRead = SPIReadResult,
        spi_write_result spi_write_result_mut: SPIWrite = SPIWriteResult,
        mcu_report mcu_report_mut: SetMCUConf = MCUReport,
        mcu_state_result mcu_state_result_mut: SetMCUState = (),
        player_lights_result player_lights_result_mut: SetPlayerLights = (),
        home_light_result home_light_result_mut: SetHomeLight = (),
        imu_mode_result imu_mode_result_mut: SetIMUMode = (),
        imu_sens_result imu_sens_result_mut: SetIMUSens = (),
        enable_vibration enable_vibration_mut: EnableVibration = (),
        maybe_accessory maybe_accessory_mut: MaybeAccessory = AccessoryResponse,
        unknown0x59 unknown0x59_mut: Unknown0x59 = (),
        unknown0x5a unknown0x5a_mut: Unknown0x5a = (),
        unknown0x5b unknown0x5b_mut: Unknown0x5b = (),
        unknown0x5c unknown0x5c_mut: Unknown0x5c = ()
    }
}

impl SubcommandReply {
    pub fn validate(&self) {
        assert!(
            self.id.try_into().is_some(),
            "invalid subcmd id{:?}",
            self.id
        )
    }

    pub fn is_spi_write_success(&self) -> Option<bool> {
        self.spi_write_result()
            .map(|r| self.ack.is_ok() && r.success())
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Default, PartialEq, Eq)]
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

#[cfg(test)]
#[test]
fn check_layout() {
    unsafe {
        let report = InputReport::new();
        assert_eq!(6, offset_of(&report, &report.u.standard_full.0.left_stick));
        assert_eq!(13, offset_of(&report, &report.u.standard_full.1));
        assert_eq!(13, offset_of(&report, &report.u.standard_subcmd.1));
        assert_eq!(15, offset_of(&report, &report.u.standard_subcmd.1.u));
        assert_eq!(49, offset_of(&report, &report.u.standard_full_mcu.2));
        assert_eq!(362, std::mem::size_of_val(&report));
    }
}
