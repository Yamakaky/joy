#[repr(C)]
#[derive(Copy, Clone)]
pub struct MCUCmd {
    pub cmd_id: MCUCmdId,
    pub subcmd_id: MCUSubCmdId,
    pub u: MCUCmdData,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union MCUCmdData {
    pub mcu_mode: MCUMode,
    pub mcu_regs: MCURegisters,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MCURegisters {
    pub len: u8,
    pub regs: [MCURegister; 9],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MCURegister {
    pub address: u16,
    pub value: u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MCUSetReg {
    pub cmd_id: MCUCmdId,
    pub subcmd_id: MCUSubCmdId,
    pub mode: MCUMode,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
// TODO: debug
pub enum MCUState {
    Suspend = 0,
    Resume = 1,
    ResumeForUpdate = 2,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum MCUMode {
    Standby = 1,
    NFC = 4,
    IR = 5,
    MaybeFWUpdate = 6,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
//TODO: unknown values
pub enum MCUCmdId {
    SetMCUMode = 0x21,
}
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
//TODO: unknown values
pub enum MCUSubCmdId {
    SetMCUMode = 0,
}
