use crate::mcu::*;
pub use ir_register::*;

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct IRRequest {
    id: RawId<IRRequestId>,
    #[allow(dead_code)]
    u: IRRequestUnion,
}

impl IRRequest {
    pub fn id(&self) -> IRRequestId {
        self.id.try_into().unwrap()
    }

    pub fn get_state() -> IRRequest {
        IRRequest {
            id: IRRequestId::GetState.into(),
            u: IRRequestUnion { nothing: () },
        }
    }
}

impl From<IRAckRequestPacket> for IRRequest {
    fn from(ack_request_packet: IRAckRequestPacket) -> Self {
        IRRequest {
            id: IRRequestId::GetSensorData.into(),
            u: IRRequestUnion { ack_request_packet },
        }
    }
}

impl From<IRReadRegisters> for IRRequest {
    fn from(read_registers: IRReadRegisters) -> Self {
        IRRequest {
            id: IRRequestId::ReadRegister.into(),
            u: IRRequestUnion { read_registers },
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, FromPrimitive, ToPrimitive)]
pub enum IRRequestId {
    GetSensorData = 0,
    GetState = 2,
    ReadRegister = 3,
}

#[repr(packed)]
#[derive(Copy, Clone)]
union IRRequestUnion {
    nothing: (),
    ack_request_packet: IRAckRequestPacket,
    read_registers: IRReadRegisters,
}

#[repr(packed)]
#[derive(Copy, Clone)]
pub struct IRAckRequestPacket {
    pub packet_missing: RawId<Bool>,
    pub missed_packet_id: u8,
    pub ack_packet_id: u8,
}

#[repr(packed)]
#[derive(Copy, Clone)]
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
#[derive(Copy, Clone)]
pub struct IRStatus {
    _unknown_0x00: u8,
    pub ir_mode: MCUIRMode,
    pub required_fw_major_version: U16LE,
    pub required_fw_minor_version: U16LE,
}

#[repr(packed)]
#[derive(Copy, Clone)]
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
        assert_eq!(11, offset_of(&report, &cmd.u.ir_request.id));
        assert_eq!(
            15,
            offset_of(&report, &cmd.u.ir_request.u.read_registers.nb_registers)
        );
    }
}
