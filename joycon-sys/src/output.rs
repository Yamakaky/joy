//! Structs binary compatible with the HID output reports
//!
//! https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/bluetooth_hid_notes.md#output-reports

use crate::{
    accessory::AccessoryCommand,
    common::*,
    imu::{self, IMUMode},
    light,
    mcu::{ir::*, *},
    raw_enum,
    spi::*,
};
use std::mem::size_of_val;

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, FromPrimitive, ToPrimitive)]
pub enum OutputReportId {
    RumbleAndSubcmd = 0x01,
    MCUFwUpdate = 0x03,
    RumbleOnly = 0x10,
    RequestMCUData = 0x11,
}

// Describes a HID report sent to the JoyCon.
//
// ```ignore
// let report = OutputReport::from(SubcommandRequest::request_device_info());
// write_hid_report(report.as_bytes());
// ```
raw_enum! {
    #[id: OutputReportId]
    #[union: OutputReportUnion]
    #[struct: OutputReport]
    pub enum OutputReportEnum {
        rumble_subcmd rumble_subcmd_mut: RumbleAndSubcmd = (Rumble, SubcommandRequest),
        mcu_fw_update mcu_fw_update_mut: MCUFwUpdate = (),
        rumble_only rumble_only_mut: RumbleOnly = Rumble,
        request_mcu_data request_mcu_data_mut: RequestMCUData = (Rumble, MCURequest)
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Rumble {
    pub packet_counter: u8,
    pub rumble_data: RumbleData,
}

impl OutputReport {
    pub fn packet_counter(&mut self) -> &mut u8 {
        unsafe { &mut self.u.rumble_only.packet_counter }
    }

    pub fn is_special(&self) -> bool {
        self.id != OutputReportId::RumbleOnly
    }

    pub fn set_registers(regs: &[ir::Register]) -> (OutputReport, &[ir::Register]) {
        let size = regs.len().min(9);
        let mut regs_fixed = [ir::Register::default(); 9];
        regs_fixed[..size].copy_from_slice(&regs[..size]);
        let mcu_cmd = MCUCommand::set_ir_registers(MCURegisters {
            len: size as u8,
            regs: regs_fixed,
        });
        (SubcommandRequest::from(mcu_cmd).into(), &regs[size..])
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
        OutputReportEnum::RumbleOnly(Rumble {
            packet_counter: 0,
            rumble_data: rumble,
        })
        .into()
    }

    pub fn len(&self) -> usize {
        match self.id.try_into() {
            Some(OutputReportId::RumbleAndSubcmd) => 49,
            Some(OutputReportId::MCUFwUpdate) => unimplemented!(),
            Some(OutputReportId::RumbleOnly) => 10,
            Some(OutputReportId::RequestMCUData) => 48,
            None => size_of_val(self),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self as *const _ as *const u8, self.len()) }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self as *mut _ as *mut u8, size_of_val(self)) }
    }

    #[cfg(test)]
    pub(crate) unsafe fn as_mcu_request(&self) -> &MCURequest {
        &self.u.request_mcu_data.1
    }

    #[cfg(test)]
    pub(crate) unsafe fn as_mcu_cmd(&self) -> &MCUCommand {
        &self.u.rumble_subcmd.1.u.set_mcu_conf
    }
}

impl From<SubcommandRequest> for OutputReport {
    fn from(subcmd: SubcommandRequest) -> Self {
        OutputReportEnum::RumbleAndSubcmd((Rumble::default(), subcmd)).into()
    }
}

impl From<SubcommandRequestEnum> for OutputReport {
    fn from(subcmd: SubcommandRequestEnum) -> Self {
        subcmd.into()
    }
}

impl From<MCURequest> for OutputReport {
    fn from(mcu_request: MCURequest) -> Self {
        OutputReportEnum::RequestMCUData((Rumble::default(), mcu_request)).into()
    }
}

//normal normal_mut: Normal = NormalInputReport,
raw_enum! {
    #[id: SubcommandId]
    #[union: SubcommandRequestUnion]
    #[struct: SubcommandRequest]
    pub enum SubcommandRequestEnum {
        get_only_controller_state get_only_controller_state_mut: GetOnlyControllerState = (),
        bluetooth_manual_pairing bluetooth_manual_pairing_mut: BluetoothManualPairing = (),
        request_device_info request_device_info_mut: RequestDeviceInfo = (),
        set_input_report_mode set_input_report_mode_mut: SetInputReportMode = RawId<InputReportId>,
        get_trigger_buttons_elapsed_time get_trigger_buttons_elapsed_time_mut: GetTriggerButtonsElapsedTime = (),
        set_shipment_mode set_shipment_mode_mut: SetShipmentMode = RawId<Bool>,
        spi_read spi_read_mut: SPIRead = SPIReadRequest,
        spi_write spi_write_mut: SPIWrite = SPIWriteRequest,
        set_mcu_conf set_mcu_conf_mut: SetMCUConf = MCUCommand,
        set_mcu_state set_mcu_state_mut: SetMCUState = RawId<MCUMode>,
        set_player_lights set_player_lights_mut: SetPlayerLights = light::PlayerLights,
        set_home_light set_home_light_mut: SetHomeLight = light::HomeLight,
        set_imu_mode set_imu_mode_mut: SetIMUMode = RawId<IMUMode>,
        set_imu_sens set_imu_sens_mut: SetIMUSens = imu::Sensitivity,
        enable_vibration enable_vibration_mut: EnableVibration = RawId<Bool>,
        maybe_accessory maybe_accessory_mut: MaybeAccessory = AccessoryCommand,
        unknown0x59 unknown0x59_mut: Unknown0x59 = (),
        unknown0x5a unknown0x5a_mut: Unknown0x5a = [u8; 38],
        unknown0x5b unknown0x5b_mut: Unknown0x5b = (),
        unknown0x5c unknown0x5c_mut: Unknown0x5c = [u8; 38]
    }
}

impl SubcommandRequest {
    pub fn disable_shipment_mode() -> Self {
        SubcommandRequestEnum::SetShipmentMode(Bool::False.into()).into()
    }

    pub fn subcmd_0x59() -> Self {
        SubcommandRequestEnum::Unknown0x59(()).into()
    }

    pub fn subcmd_0x5a() -> Self {
        SubcommandRequestEnum::Unknown0x5a([
            4, 1, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
        ])
        .into()
    }

    pub fn subcmd_0x5b() -> Self {
        SubcommandRequestEnum::Unknown0x5b(()).into()
    }

    pub fn subcmd_0x5c_0() -> Self {
        SubcommandRequestEnum::Unknown0x5c([
            0, 0, 150, 227, 28, 0, 0, 0, 236, 153, 172, 227, 28, 0, 0, 0, 243, 130, 241, 89, 46,
            89, 0, 0, 224, 88, 179, 227, 28, 0, 0, 0, 0, 242, 5, 42, 1, 0,
        ])
        .into()
    }

    pub fn subcmd_0x5c_6() -> Self {
        SubcommandRequestEnum::Unknown0x5c([
            6, 3, 37, 6, 0, 0, 0, 0, 236, 153, 172, 227, 28, 0, 0, 0, 105, 155, 22, 246, 93, 86, 0,
            0, 4, 0, 0, 0, 0, 0, 0, 0, 144, 40, 161, 227, 28, 0,
        ])
        .into()
    }
}

impl From<MCUCommand> for SubcommandRequest {
    fn from(mcu_cmd: MCUCommand) -> Self {
        SubcommandRequestEnum::SetMCUConf(mcu_cmd).into()
    }
}

impl From<AccessoryCommand> for SubcommandRequest {
    fn from(accessory_cmd: AccessoryCommand) -> Self {
        SubcommandRequestEnum::MaybeAccessory(accessory_cmd).into()
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

impl From<crate::imu::Sensitivity> for SubcommandRequest {
    fn from(imu_sensitivity: crate::imu::Sensitivity) -> Self {
        SubcommandRequestEnum::SetIMUSens(imu_sensitivity).into()
    }
}

impl From<SPIReadRequest> for SubcommandRequest {
    fn from(spi_read: SPIReadRequest) -> Self {
        SubcommandRequestEnum::SPIRead(spi_read).into()
    }
}

impl From<SPIWriteRequest> for SubcommandRequest {
    fn from(spi_write: SPIWriteRequest) -> Self {
        SubcommandRequestEnum::SPIWrite(spi_write).into()
    }
}

impl From<light::PlayerLights> for SubcommandRequest {
    fn from(player_lights: light::PlayerLights) -> Self {
        SubcommandRequestEnum::SetPlayerLights(player_lights).into()
    }
}

impl From<light::HomeLight> for SubcommandRequest {
    fn from(home_light: light::HomeLight) -> Self {
        SubcommandRequestEnum::SetHomeLight(home_light).into()
    }
}

#[cfg(test)]
#[test]
pub fn check_layout() {
    unsafe {
        let report = OutputReport::new();
        assert_eq!(2, offset_of(&report, &report.u.rumble_only.rumble_data));
        assert_eq!(10, offset_of(&report, &report.u.rumble_subcmd.1));
        assert_eq!(11, offset_of(&report, report.as_mcu_cmd()));
        assert_eq!(49, std::mem::size_of_val(&report));
    }
}
