use crate::mcu::*;
pub use ir_register::*;

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, FromPrimitive, ToPrimitive)]
pub enum IRRequestId {
    GetSensorData = 0,
    GetState = 2,
    ReadRegister = 3,
}

raw_enum! {
    #[id: IRRequestId]
    #[union: IRRequestUnion]
    #[struct: IRRequest]
    pub enum IRRequestEnum {
        get_sensor_data get_sensor_data_mut: GetSensorData = IRAckRequestPacket,
        get_state get_state_mut: GetState = (),
        read_register read_register_mut: ReadRegister = IRReadRegisters
    }
}

impl From<IRAckRequestPacket> for IRRequest {
    fn from(ack_request_packet: IRAckRequestPacket) -> Self {
        IRRequest {
            id: IRRequestId::GetSensorData.into(),
            u: IRRequestUnion {
                get_sensor_data: ack_request_packet,
            },
        }
    }
}

impl From<IRReadRegisters> for IRRequest {
    fn from(read_registers: IRReadRegisters) -> Self {
        IRRequest {
            id: IRRequestId::ReadRegister.into(),
            u: IRRequestUnion {
                read_register: read_registers,
            },
        }
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct IRAckRequestPacket {
    pub packet_missing: RawId<Bool>,
    pub missed_packet_id: u8,
    pub ack_packet_id: u8,
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct IRReadRegisters {
    pub unknown_0x01: u8,
    pub page: u8,
    pub offset: u8,
    pub nb_registers: u8,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, FromPrimitive, ToPrimitive)]
pub enum MCUIRMode {
    IRSensorReset = 0,
    IRSensorSleep = 1,
    WaitingForConfigurationMaybe = 2,
    Moment = 3,
    /// Wii-style pointing
    Dpd = 4,
    Unknown5 = 5,
    Clustering = 6,
    ImageTransfer = 7,
    HandAnalysisSilhouette = 8,
    HandAnalysisImage = 9,
    HandAnalysisSilhouetteImage = 10,
    Unknown11 = 11,
    /// Used in ringfit for pulse rate detection
    PulseRate = 0xd,
}

#[repr(packed)]
#[derive(Debug, Copy, Clone)]
pub struct MCUIRModeData {
    pub ir_mode: RawId<MCUIRMode>,
    /// Set number of packets to output per buffer
    pub no_of_frags: u8,
    /// Get it from MCUStatus
    pub mcu_fw_version: (U16LE, U16LE),
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct IRStatus {
    _unknown_0x00: u8,
    pub ir_mode: RawId<MCUIRMode>,
    pub required_fw_major_version: U16LE,
    pub required_fw_minor_version: U16LE,
}

// TODO: better debug
#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct IRRegistersSlice {
    _unknown_0x00: u8,
    pub page: u8,
    pub offset: u8,
    pub nb_registers: u8,
    pub values: [u8; 0x7f],
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct IRData {
    _unknown: [u8; 2],
    pub frag_number: u8,
    pub average_intensity: u8,
    // Only when EXFilter enabled
    _unknown3: u8,
    pub white_pixel_count: U16LE,
    pub ambient_noise_count: U16LE,
    pub img_fragment: [u8; 300],
}

impl fmt::Debug for IRData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("IRData")
            .field("frag_number", &self.frag_number)
            .field("average_intensity", &self.average_intensity)
            .field("white_pixel_count", &self.white_pixel_count)
            .field("ambient_noise_count", &self.ambient_noise_count)
            .finish()
    }
}

#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct MCURegisters {
    pub len: u8,
    pub regs: [ir_register::Register; 9],
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct MCUSetReg {
    pub cmd_id: MCUCommandId,
    pub subcmd_id: MCUSubCommandId,
    pub mode: RawId<MCUMode>,
}

#[cfg(test)]
#[test]
fn check_output_layout() {
    unsafe {
        let report = crate::output::OutputReport::new();
        let cmd = report.as_mcu_request();
        assert_eq!(10, offset_of(&report, cmd));
        assert_eq!(11, offset_of(&report, &cmd.u.get_ir_data));
        assert_eq!(
            15,
            offset_of(&report, &cmd.u.get_ir_data.u.read_register.nb_registers)
        );
    }
}
