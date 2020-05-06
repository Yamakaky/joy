use anyhow::Result;
use joycon_sys::input::*;
use joycon_sys::output::*;
use joycon_sys::*;
use std::mem::{size_of, size_of_val};

pub struct JoyCon {
    device: hidapi::HidDevice,
    info: hidapi::DeviceInfo,
    counter: u8,
}

impl JoyCon {
    pub fn new(device: hidapi::HidDevice, info: hidapi::DeviceInfo) -> JoyCon {
        assert!([
            JOYCON_L_BT,
            JOYCON_R_BT,
            PRO_CONTROLLER,
            JOYCON_CHARGING_GRIP,
        ]
        .contains(&info.product_id()));
        JoyCon {
            device,
            info,
            counter: 42,
        }
    }
    pub fn send(&mut self, report: &mut OutputReport) -> Result<()> {
        report.packet_counter = self.counter;
        self.counter += 1;
        let raw_data = unsafe {
            std::slice::from_raw_parts(
                (report as *const OutputReport).cast::<u8>(),
                size_of_val(report),
            )
        };
        let nb_written = self.device.write(raw_data)?;
        // TODO: check that, always true
        assert_ne!(size_of::<InputReport>(), 49);
        assert_eq!(nb_written, 49);
        Ok(())
    }

    pub fn recv(&self) -> Result<InputReport> {
        let mut buffer = [0u8; size_of::<InputReport>()];
        let nb_read = self.device.read(&mut buffer)?;
        assert_eq!(nb_read, buffer.len());
        Ok(unsafe { std::mem::transmute(buffer) })
    }

    pub fn print_dev_info(&mut self) -> Result<DeviceInfo> {
        // enable IMU
        let info = self.send_subcmd_wait(OutputReport {
            packet_counter: 0,
            report_id: OutputReportId::RumbleSubcmd,
            rumble_data: RumbleData::default(),
            subcmd: SubcommandRequest {
                subcommand_id: SubcommandId::RequestDeviceInfo,
                u: SubcommandRequestData { nothing: () },
            },
        })?;
        Ok(unsafe { info.u.device_info })
    }

    pub fn enable_imu(&mut self) -> Result<()> {
        // enable IMU
        self.send_subcmd_wait(OutputReport {
            packet_counter: 0,
            report_id: OutputReportId::RumbleSubcmd,
            rumble_data: RumbleData::default(),
            subcmd: SubcommandRequest {
                subcommand_id: SubcommandId::EnableIMU,
                u: SubcommandRequestData { nothing: () },
            },
        })?;
        Ok(())
    }

    pub fn set_standard_mode(&mut self) -> Result<()> {
        self.send_subcmd_wait(OutputReport {
            packet_counter: 0,
            report_id: OutputReportId::RumbleSubcmd,
            rumble_data: RumbleData::default(),
            subcmd: SubcommandRequest {
                subcommand_id: SubcommandId::SetInputReportMode,
                u: SubcommandRequestData {
                    input_report_mode: InputReportMode::StandardFull,
                },
            },
        })?;
        Ok(())
    }

    pub fn set_nfc_ir_mode(&mut self) -> Result<()> {
        self.send_subcmd_wait(OutputReport {
            packet_counter: 0,
            report_id: OutputReportId::RumbleSubcmd,
            rumble_data: RumbleData::default(),
            subcmd: SubcommandRequest {
                subcommand_id: SubcommandId::SetInputReportMode,
                u: SubcommandRequestData {
                    input_report_mode: InputReportMode::NFCIR,
                },
            },
        })?;
        Ok(())
    }

    pub fn enable_mcu(&mut self) -> Result<()> {
        self.send_subcmd_wait(OutputReport {
            packet_counter: 0,
            report_id: OutputReportId::RumbleSubcmd,
            rumble_data: RumbleData::default(),
            subcmd: SubcommandRequest {
                subcommand_id: SubcommandId::SetMCUState,
                u: SubcommandRequestData {
                    mcu_state: MCUState::Resume,
                },
            },
        })?;
        Ok(())
    }

    pub fn disable_mcu(&mut self) -> Result<()> {
        self.send_subcmd_wait(OutputReport {
            packet_counter: 0,
            report_id: OutputReportId::RumbleSubcmd,
            rumble_data: RumbleData::default(),
            subcmd: SubcommandRequest {
                subcommand_id: SubcommandId::SetMCUState,
                u: SubcommandRequestData {
                    mcu_state: MCUState::Suspend,
                },
            },
        })?;
        Ok(())
    }

    pub fn set_player_light(&mut self, player_lights: PlayerLights) -> Result<()> {
        self.send_subcmd_wait(OutputReport {
            packet_counter: 0,
            report_id: OutputReportId::RumbleSubcmd,
            rumble_data: RumbleData::default(),
            subcmd: SubcommandRequest {
                subcommand_id: SubcommandId::SetPlayerLights,
                u: SubcommandRequestData { player_lights },
            },
        })?;
        Ok(())
    }

    fn send_subcmd_wait(&mut self, mut out_report: OutputReport) -> Result<SubcommandReply> {
        self.send(&mut out_report)?;
        // TODO: loop limit
        loop {
            let in_report = self.recv()?;
            unsafe {
                if in_report.report_id.try_into().unwrap() == InputReportId::Standard {
                    let subcmd_reply = in_report.u.standard.u.subcmd_reply;
                    if subcmd_reply.subcommand_id.try_into().unwrap()
                        == out_report.subcmd.subcommand_id
                    {
                        return Ok(subcmd_reply);
                    }
                }
            }
        }
    }
}

impl std::fmt::Debug for JoyCon {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("JoyCon")
            .field("manufacturer", &self.info.manufacturer_string())
            .field("product", &self.info.product_string())
            .field("serial", &self.info.serial_number())
            .finish()
    }
}
