#![allow(dead_code)]

use crate::calibration::Calibration;
use anyhow::{bail, ensure, Context, Result};
use joycon_sys::input::*;
use joycon_sys::mcu::*;
use joycon_sys::output::*;
use joycon_sys::spi::*;
use joycon_sys::*;

/// 200 samples per second with 3 sample per InputReport.
pub const IMU_SAMPLES_PER_SECOND: u32 = 200;

pub struct JoyCon {
    device: hidapi::HidDevice,
    info: hidapi::DeviceInfo,
    counter: u8,
    calib_gyro: Calibration,
    gyro_sens: GyroSens,
    calib_accel: Calibration,
    accel_sens: AccSens,
    pub max_raw_gyro: i16,
    pub max_raw_accel: i16,
    left_stick_calib: StickCalibration,
    right_stick_calib: StickCalibration,
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
            counter: 0,
            // 10s with 3 reports at 60Hz
            calib_gyro: Calibration::new(10 * IMU_SAMPLES_PER_SECOND as usize),
            gyro_sens: GyroSens::DPS2000,
            calib_accel: Calibration::new(10 * IMU_SAMPLES_PER_SECOND as usize),
            accel_sens: AccSens::G8,
            max_raw_gyro: 0,
            max_raw_accel: 0,
            left_stick_calib: StickCalibration::default(),
            right_stick_calib: StickCalibration::default(),
        }
    }
    pub fn send(&mut self, report: &mut OutputReport) -> Result<()> {
        report.packet_counter = self.counter;
        self.counter = (self.counter + 1) & 0xf;
        let buffer = report.as_bytes();
        let nb_written = self.device.write(buffer)?;
        assert_eq!(nb_written, buffer.len());
        Ok(())
    }

    pub fn recv(&self) -> Result<InputReport> {
        // Larger buffer to detect unhandled received data
        let mut reports = [InputReport::new(); 2];
        let buffer = unsafe {
            std::slice::from_raw_parts_mut(
                &mut reports as *mut _ as *mut u8,
                std::mem::size_of::<InputReport>(),
            )
        };
        let nb_read = self.device.read(buffer)?;
        assert_eq!(nb_read, std::mem::size_of_val(&reports[0]));
        Ok(reports[0])
    }

    pub fn load_calibration(&mut self) -> Result<()> {
        let factory_result = self.read_spi(RANGE_FACTORY_CALIBRATION_SENSORS)?;
        let factory_settings = factory_result.imu_factory_calib().unwrap();
        self.calib_accel.factory_offset = factory_settings.acc_offset();
        self.calib_gyro.factory_offset = factory_settings.gyro_offset();

        let user_result = self.read_spi(RANGE_USER_CALIBRATION_SENSORS)?;
        let user_settings = user_result.imu_user_calib().unwrap();
        self.calib_accel.user_offset = user_settings.acc_offset();
        self.calib_gyro.user_offset = user_settings.gyro_offset();

        let factory_result = self.read_spi(RANGE_FACTORY_CALIBRATION_STICKS)?;
        let factory_settings = factory_result.sticks_factory_calib().unwrap();
        let user_result = self.read_spi(RANGE_USER_CALIBRATION_STICKS)?;
        let user_settings = user_result.sticks_user_calib().unwrap();
        self.left_stick_calib = user_settings.left.calib().unwrap_or(factory_settings.left);
        self.right_stick_calib = user_settings
            .right
            .calib()
            .unwrap_or(factory_settings.right);

        Ok(())
    }

    pub fn set_imu_sens(&mut self) -> Result<()> {
        let gyro_sens = GyroSens::DPS2000;
        let accel_sens = AccSens::G8;
        self.send_subcmd_wait(
            OutputReportId::RumbleAndSubcmd,
            SubcommandRequest {
                subcommand_id: SubcommandId::SetIMUSens,
                u: SubcommandRequestData {
                    imu_sensitivity: IMUSensitivity {
                        gyro_sens,
                        acc_sens: accel_sens,
                        ..IMUSensitivity::default()
                    },
                },
            },
        )?;
        self.gyro_sens = gyro_sens;
        self.accel_sens = accel_sens;
        Ok(())
    }

    pub fn get_dev_info(&mut self) -> Result<DeviceInfo> {
        let reply = self.send_subcmd_wait(
            OutputReportId::RumbleAndSubcmd,
            SubcommandRequest {
                subcommand_id: SubcommandId::RequestDeviceInfo,
                u: SubcommandRequestData { nothing: () },
            },
        )?;
        Ok(*reply.device_info().unwrap())
    }

    pub fn enable_imu(&mut self) -> Result<()> {
        self.send_subcmd_wait(
            OutputReportId::RumbleAndSubcmd,
            SubcommandRequest {
                subcommand_id: SubcommandId::EnableIMU,
                u: SubcommandRequestData { imu_enabled: true },
            },
        )?;
        Ok(())
    }

    pub fn set_report_mode_standard(&mut self) -> Result<()> {
        self.send_subcmd_wait(
            OutputReportId::RumbleAndSubcmd,
            SubcommandRequest {
                subcommand_id: SubcommandId::SetInputReportMode,
                u: SubcommandRequestData {
                    input_report_mode: InputReportId::StandardFull,
                },
            },
        )?;
        Ok(())
    }

    pub fn set_report_mode_mcu(&mut self) -> Result<()> {
        self.send_subcmd_wait(
            OutputReportId::RumbleAndSubcmd,
            SubcommandRequest {
                subcommand_id: SubcommandId::SetInputReportMode,
                u: SubcommandRequestData {
                    input_report_mode: InputReportId::StandardFullMCU,
                },
            },
        )?;
        Ok(())
    }

    pub fn enable_mcu(&mut self) -> Result<()> {
        self.send_subcmd_wait(
            OutputReportId::RumbleAndSubcmd,
            SubcommandRequest {
                subcommand_id: SubcommandId::SetMCUState,
                u: SubcommandRequestData {
                    mcu_mode: MCUMode::Standby,
                },
            },
        )?;
        self.wait_mcu_status(MCUMode::Standby)
            .context("enable_mcu")?;
        Ok(())
    }

    fn wait_mcu_status(&mut self, mode: MCUMode) -> Result<()> {
        // The MCU takes some time to warm up so we retry until we get an answer
        for _ in 0..20 {
            // TODO: loop limit
            self.send_mcu_subcmd(MCUSubcommand {
                subcmd_id: MCUSubCmdId2::GetStatus,
                u: MCUSubcommandUnion { nothing: () },
            })?;
            for _ in 0..8 {
                let in_report = self.recv()?;
                if let Some(mcu_report) = in_report.mcu_report() {
                    if let Some(status) = mcu_report.as_status() {
                        if status.state == mode {
                            return Ok(());
                        }
                    }
                }
            }
        }
        bail!("error getting the MCU status: timeout");
    }

    pub fn disable_mcu(&mut self) -> Result<()> {
        self.send_subcmd_wait(
            OutputReportId::RumbleAndSubcmd,
            SubcommandRequest {
                subcommand_id: SubcommandId::SetMCUState,
                u: SubcommandRequestData {
                    mcu_mode: MCUMode::Suspend,
                },
            },
        )?;
        Ok(())
    }

    pub fn set_mcu_mode_ir(&mut self) -> Result<()> {
        let mut mcu_cmd = MCUCmd {
            cmd_id: MCUCmdId::SetMCUMode,
            subcmd_id: MCUSubCmdId::SetMCUMode,
            u: MCUCmdData {
                mcu_mode: MCUMode::IR,
            },
        };
        mcu_cmd.compute_crc();
        self.send_subcmd_wait(
            OutputReportId::RumbleAndSubcmd,
            SubcommandRequest {
                subcommand_id: SubcommandId::SetMCUConf,
                u: SubcommandRequestData { mcu_cmd },
            },
        )?;
        self.wait_mcu_status(MCUMode::IR)
            .context("set_mcu_mode_ir")?;
        Ok(())
    }

    pub fn set_player_light(&mut self, player_lights: PlayerLights) -> Result<()> {
        self.send_subcmd_wait(
            OutputReportId::RumbleAndSubcmd,
            SubcommandRequest {
                subcommand_id: SubcommandId::SetPlayerLights,
                u: SubcommandRequestData { player_lights },
            },
        )?;
        Ok(())
    }

    fn send_subcmd_wait(
        &mut self,
        report_id: OutputReportId,
        subcmd: SubcommandRequest,
    ) -> Result<SubcommandReply> {
        let mut out_report = OutputReport {
            packet_counter: 0,
            report_id,
            rumble_data: RumbleData::default(),
            u: SubcommandRequestUnion { subcmd },
        };
        self.send(&mut out_report)?;
        // TODO: loop limit
        loop {
            let in_report = self.recv()?;
            if let Some(reply) = in_report.subcmd_reply() {
                if reply.id() == Some(subcmd.subcommand_id) {
                    ensure!(reply.ack.is_ok(), "subcmd reply is nack");
                    return Ok(*reply);
                }
            }
        }
    }

    fn send_mcu_subcmd(&mut self, mcu_subcmd: MCUSubcommand) -> Result<()> {
        let mut out_report = OutputReport {
            packet_counter: 0,
            report_id: OutputReportId::RequestMCUData,
            rumble_data: RumbleData::default(),
            u: SubcommandRequestUnion { mcu_subcmd },
        };
        self.send(&mut out_report)?;
        Ok(())
    }

    fn read_spi(&mut self, range: SPIRange) -> Result<SPIReadResult> {
        let reply = self.send_subcmd_wait(
            OutputReportId::RumbleAndSubcmd,
            SubcommandRequest {
                subcommand_id: SubcommandId::SPIRead,
                u: SubcommandRequestData {
                    spi_read: SPIReadRequest::new(range),
                },
            },
        )?;
        let result = reply.spi_result().unwrap();
        ensure!(
            range == result.range(),
            "invalid range {:?}",
            result.range()
        );
        Ok(*result)
    }

    pub fn get_sticks(&mut self) -> Result<((f32, f32), (f32, f32))> {
        let report = self.recv()?;
        let inputs = report.standard().expect("should be standard");
        Ok((
            self.left_stick_calib
                .value_from_raw(inputs.left_stick.x(), inputs.left_stick.y()),
            self.right_stick_calib
                .value_from_raw(inputs.right_stick.x(), inputs.right_stick.y()),
        ))
    }

    pub fn get_gyro_rot_delta(&mut self, apply_calibration: bool) -> Result<[Vector3; 3]> {
        let report = self.recv()?;
        let gyro_frames = report.imu_frames().expect("no imu frame received");
        let offset = self
            .calib_gyro
            .user_offset
            .unwrap_or(self.calib_gyro.factory_offset);
        let mut out = [Vector3::default(); 3];
        // frames are from newest to oldest so we iter backward
        for (frame, out) in gyro_frames.iter().rev().zip(out.iter_mut()) {
            let max = [
                frame.raw_gyro().0.abs() as i16,
                frame.raw_gyro().1.abs() as i16,
                frame.raw_gyro().2.abs() as i16,
            ]
            .iter()
            .cloned()
            .max()
            .unwrap();
            self.max_raw_gyro = self.max_raw_gyro.max(max);
            if max > i16::MAX - 1000 {
                println!("saturation");
            }

            let gyro_rps = frame.gyro_rps(offset, self.gyro_sens) / IMU_SAMPLES_PER_SECOND as f32;
            *out = if apply_calibration {
                gyro_rps - self.calib_gyro.get_average()
            } else {
                gyro_rps
            }
        }
        Ok(out)
    }

    pub fn get_accel_delta_g(&mut self, apply_calibration: bool) -> Result<[Vector3; 3]> {
        let report = self.recv()?;
        let frames = report.imu_frames().expect("no imu frame received");
        let offset = self
            .calib_accel
            .user_offset
            .unwrap_or(self.calib_accel.factory_offset);
        let mut out = [Vector3::default(); 3];
        // frames are from newest to oldest so we iter backward
        for (frame, out) in frames.iter().rev().zip(out.iter_mut()) {
            let max = [
                frame.raw_accel().0.abs() as i16,
                frame.raw_accel().1.abs() as i16,
                frame.raw_accel().2.abs() as i16,
            ]
            .iter()
            .cloned()
            .max()
            .unwrap();
            self.max_raw_accel = self.max_raw_accel.max(max);
            if max > i16::MAX - 1000 {
                println!("saturation");
            }

            let accel_g = frame.accel_g(offset, self.accel_sens);
            *out = if apply_calibration {
                accel_g - self.calib_accel.get_average()
            } else {
                accel_g
            }
        }
        Ok(out)
    }

    pub fn reset_calibration(&mut self) -> Result<()> {
        // seems needed
        self.get_gyro_rot_delta(false)?;
        self.calib_gyro.reset();
        for _ in 0..60 {
            for frame in &self.get_gyro_rot_delta(false)? {
                self.calib_gyro.push(*frame);
            }
        }
        Ok(())
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
