use crate::calibration::Calibration;
use anyhow::{ensure, Result};
use joycon_sys::input::*;
use joycon_sys::output::*;
use joycon_sys::spi::*;
use joycon_sys::*;
use std::mem::{size_of, size_of_val};

pub const GYRO_SAMPLES_PER_SECOND: u32 = 200 / 3;

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
            // 10s with 3 reports at 60Hz
            calib_gyro: Calibration::new(10 * GYRO_SAMPLES_PER_SECOND as usize),
            gyro_sens: GyroSens::DPS2000,
            calib_accel: Calibration::new(10 * GYRO_SAMPLES_PER_SECOND as usize),
            accel_sens: AccSens::G8,
            max_raw_gyro: 0,
            max_raw_accel: 0,
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

    pub fn load_calibration(&mut self) -> Result<(&Calibration, &Calibration)> {
        let factory = self.read_spi(RANGE_FACTORY_CALIBRATION_SENSORS)?;
        let factory_settings = unsafe { factory.factory_calib };
        self.calib_accel.factory_offset = factory_settings.acc_offset();
        self.calib_gyro.factory_offset = factory_settings.gyro_offset();
        let user = self.read_spi(RANGE_USER_CALIBRATION_SENSORS)?;
        let user_settings = unsafe { user.user_calib };
        self.calib_accel.user_offset = user_settings.acc_offset();
        self.calib_gyro.user_offset = user_settings.gyro_offset();
        Ok((&self.calib_gyro, &self.calib_accel))
    }

    pub fn set_imu_sens(&mut self) -> Result<()> {
        let gyro_sens = GyroSens::DPS2000;
        let accel_sens = AccSens::G8;
        self.send_subcmd_wait(
            OutputReportId::RumbleSubcmd,
            SubcommandRequest {
                subcommand_id: SubcommandId::SetIMUSens,
                u: SubcommandRequestData {
                    imu_sensitivity: IMUSensitivity {
                        gyro_sens,
                        acc_sens: accel_sens,
                        gyro_perf_rate: GyroPerfRate::Hz833,
                        acc_anti_aliasing: AccAntiAliasing::Hz100,
                    },
                },
            },
        )?;
        self.gyro_sens = gyro_sens;
        self.accel_sens = accel_sens;
        Ok(())
    }

    pub fn get_dev_info(&mut self) -> Result<DeviceInfo> {
        let info = self.send_subcmd_wait(
            OutputReportId::RumbleSubcmd,
            SubcommandRequest {
                subcommand_id: SubcommandId::RequestDeviceInfo,
                u: SubcommandRequestData { nothing: () },
            },
        )?;
        Ok(unsafe { info.u.device_info })
    }

    pub fn enable_imu(&mut self) -> Result<()> {
        self.send_subcmd_wait(
            OutputReportId::RumbleSubcmd,
            SubcommandRequest {
                subcommand_id: SubcommandId::EnableIMU,
                u: SubcommandRequestData { imu_enabled: true },
            },
        )?;
        Ok(())
    }

    pub fn set_standard_mode(&mut self) -> Result<()> {
        self.send_subcmd_wait(
            OutputReportId::RumbleSubcmd,
            SubcommandRequest {
                subcommand_id: SubcommandId::SetInputReportMode,
                u: SubcommandRequestData {
                    input_report_mode: InputReportMode::StandardFull,
                },
            },
        )?;
        Ok(())
    }

    pub fn set_nfc_ir_mode(&mut self) -> Result<()> {
        self.send_subcmd_wait(
            OutputReportId::RumbleSubcmd,
            SubcommandRequest {
                subcommand_id: SubcommandId::SetInputReportMode,
                u: SubcommandRequestData {
                    input_report_mode: InputReportMode::NFCIR,
                },
            },
        )?;
        Ok(())
    }

    pub fn enable_mcu(&mut self) -> Result<()> {
        self.send_subcmd_wait(
            OutputReportId::RumbleSubcmd,
            SubcommandRequest {
                subcommand_id: SubcommandId::SetMCUState,
                u: SubcommandRequestData {
                    mcu_state: MCUState::Resume,
                },
            },
        )?;
        Ok(())
    }

    pub fn disable_mcu(&mut self) -> Result<()> {
        self.send_subcmd_wait(
            OutputReportId::RumbleSubcmd,
            SubcommandRequest {
                subcommand_id: SubcommandId::SetMCUState,
                u: SubcommandRequestData {
                    mcu_state: MCUState::Suspend,
                },
            },
        )?;
        Ok(())
    }

    pub fn set_player_light(&mut self, player_lights: PlayerLights) -> Result<()> {
        self.send_subcmd_wait(
            OutputReportId::RumbleSubcmd,
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
            subcmd,
        };
        self.send(&mut out_report)?;
        // TODO: loop limit
        // TODO: check ACK
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

    fn read_spi(&mut self, range: SPIRange) -> Result<SPIResultData> {
        let reply = self.send_subcmd_wait(
            OutputReportId::RumbleSubcmd,
            SubcommandRequest {
                subcommand_id: SubcommandId::SPIRead,
                u: SubcommandRequestData {
                    spi_read: SPIReadRequest::new(range),
                },
            },
        )?;
        let result = unsafe { reply.u.spi_read };
        ensure!(
            range == result.range(),
            "invalid range {:?}",
            result.range()
        );
        Ok(result.data)
    }

    pub fn get_gyro_rot_delta(&mut self, apply_calibration: bool) -> Result<[Vector3; 3]> {
        let report = self.recv()?;

        ensure!(
            report.report_id == InputReportId::StandardFull,
            "expected StandardFull, got {:?}",
            report.report_id
        );

        let report = unsafe { report.u.standard };
        let gyro_frames = unsafe { report.u.gyro_acc_nfc_ir.gyro_acc_frames };
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

            let gyro_rps = frame.gyro_rps(offset, self.gyro_sens) / GYRO_SAMPLES_PER_SECOND as f32;
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

        ensure!(
            report.report_id == InputReportId::StandardFull,
            "expected StandardFull, got {:?}",
            report.report_id
        );

        let report = unsafe { report.u.standard };
        let frames = unsafe { report.u.gyro_acc_nfc_ir.gyro_acc_frames };
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
