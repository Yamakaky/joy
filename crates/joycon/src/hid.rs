use std::convert::TryInto;

use crate::imu_handler;
use anyhow::{bail, ensure, Context, Result};
use cgmath::Vector2;
use joycon_sys::mcu::*;
use joycon_sys::output::*;
use joycon_sys::spi::*;
use joycon_sys::*;
use joycon_sys::{imu::IMUMode, mcu::ir::*};
use joycon_sys::{input::*, light};
use tracing::{field::debug, instrument, trace, Span};

const WAIT_TIMEOUT: u32 = 200;

#[derive(Debug, Clone)]
pub struct Report {
    pub left_stick: Vector2<f64>,
    pub right_stick: Vector2<f64>,
    pub buttons: ButtonsStatus,
    pub info: DeviceStatus,
    #[cfg(feature = "ir")]
    pub image: Option<image::GrayImage>,
    pub imu: Option<[imu_handler::IMU; 3]>,
    pub raw: InputReport,
}

pub struct JoyCon {
    device: hidapi::HidDevice,
    info: hidapi::DeviceInfo,
    counter: u8,
    pub max_raw_gyro: i16,
    pub max_raw_accel: i16,
    left_stick_calib: LeftStickCalibration,
    right_stick_calib: RightStickCalibration,
    #[cfg(feature = "ir")]
    image: crate::image::Image,
    enable_ir_loop: bool,
    imu_handler: crate::imu_handler::Handler,
    device_type: WhichController,
}

impl JoyCon {
    #[instrument(level = "info", skip(device), err)]
    pub fn new(device: hidapi::HidDevice, info: hidapi::DeviceInfo) -> Result<JoyCon> {
        let device_type = WhichController::from_product_id(info.product_id())?;
        let mut joycon = JoyCon {
            device,
            info,
            counter: 0,
            max_raw_gyro: 0,
            max_raw_accel: 0,
            left_stick_calib: LeftStickCalibration::default(),
            right_stick_calib: RightStickCalibration::default(),
            #[cfg(feature = "ir")]
            image: crate::image::Image::new(),
            enable_ir_loop: false,
            imu_handler: crate::imu_handler::Handler::new(
                device_type,
                imu::GyroSens::default(),
                imu::AccSens::default(),
            ),
            device_type,
        };

        joycon.call_subcmd_wait(SubcommandRequest::disable_shipment_mode())?;
        joycon.set_report_mode_standard()?;
        Ok(joycon)
    }

    pub fn supports_ir(&self) -> bool {
        self.device_type == WhichController::RightJoyCon
    }

    #[instrument(level = "trace", skip(self), fields(special))]
    pub fn send(&mut self, report: &mut OutputReport) -> Result<()> {
        *report.packet_counter() = self.counter;
        self.counter = (self.counter + 1) & 0xf;
        Span::current().record("special", &report.is_special());
        trace!(out_report = %hex::encode(report.as_bytes()));
        let nb_written = self.device.write(report.as_bytes())?;
        assert_eq!(nb_written, report.byte_size());
        Ok(())
    }

    #[instrument(level = "trace", skip(self), fields(special, report))]
    pub fn recv(&mut self) -> Result<InputReport> {
        let mut report = InputReport::new();
        let nb_read = self.device.read(report.as_bytes_mut())?;
        assert!(nb_read >= report.len(), "{} < {}", nb_read, report.len());
        Span::current()
            .record("special", &report.is_special())
            .record("report", &debug(report));
        trace!(in__report = %hex::encode(report.as_bytes()));
        report.validate();
        if let Some(frames) = report.imu_frames() {
            self.imu_handler.handle_frames(frames);
        }
        #[cfg(feature = "ir")]
        if let Some(mcu_report) = report.mcu_report() {
            if self.enable_ir_loop {
                for packet in self.image.handle(mcu_report).iter_mut().flatten() {
                    self.send(packet)?;
                }
            }
        }
        Ok(report)
    }

    pub fn set_rumble(&mut self, rumble: RumbleData) -> Result<()> {
        self.send(&mut OutputReport::set_rumble(rumble))?;
        Ok(())
    }

    pub fn tick(&mut self) -> Result<Report> {
        let report = self.recv()?;
        let std_report = report.standard().expect("should be standard");

        let left_stick = self
            .left_stick_calib
            .value_from_raw(std_report.left_stick.x(), std_report.left_stick.y());
        let right_stick = self
            .right_stick_calib
            .value_from_raw(std_report.right_stick.x(), std_report.right_stick.y());

        Ok(Report {
            left_stick,
            right_stick,
            buttons: std_report.buttons,
            info: std_report.info,
            #[cfg(feature = "ir")]
            image: self.image.last_image.take(),
            imu: report
                .imu_frames()
                .map(|f| self.imu_handler.handle_frames(f)),
            raw: report,
        })
    }

    pub fn load_calibration(&mut self) -> Result<()> {
        let factor_sensor_calib = self.read_spi()?;
        self.imu_handler.set_factory(factor_sensor_calib);

        let user_sensor_calib = self.read_spi()?;
        self.imu_handler.set_user(user_sensor_calib);

        self.imu_handler.reset_calibration();

        let factory_settings: SticksCalibration = self.read_spi()?;
        //let user_result = self.read_spi(RANGE_USER_CALIBRATION_STICKS)?;
        //let user_settings = user_result.sticks_user_calib().unwrap();
        //self.left_stick_calib = user_settings.left.calib().unwrap_or(factory_settings.left);
        //self.right_stick_calib = user_settings
        //    .right
        //    .calib()
        //    .unwrap_or(factory_settings.right);
        // TODO: fix
        self.left_stick_calib = factory_settings.left;
        self.right_stick_calib = factory_settings.right;

        Ok(())
    }

    #[instrument(level = "info", skip(self), err)]
    pub fn get_dev_info(&mut self) -> Result<DeviceInfo> {
        let reply = self.call_subcmd_wait(SubcommandRequestEnum::RequestDeviceInfo(()))?;
        Ok(*reply.device_info().unwrap())
    }

    #[instrument(level = "info", skip(self), err)]
    pub fn set_home_light(&mut self, home_light: light::HomeLight) -> Result<()> {
        self.call_subcmd_wait(home_light)?;
        Ok(())
    }

    #[instrument(level = "info", skip(self), err)]
    pub fn set_player_light(&mut self, player_lights: light::PlayerLights) -> Result<()> {
        self.call_subcmd_wait(player_lights)?;
        Ok(())
    }

    #[instrument(level = "info", skip(self), err)]
    fn set_report_mode_standard(&mut self) -> Result<()> {
        self.call_subcmd_wait(SubcommandRequestEnum::SetInputReportMode(
            InputReportId::StandardFull.into(),
        ))?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    pub fn call_subcmd_wait<S: Into<SubcommandRequest> + std::fmt::Debug>(
        &mut self,
        subcmd: S,
    ) -> Result<SubcommandReply> {
        let subcmd = subcmd.into();
        let mut out_report = subcmd.into();

        self.send(&mut out_report)?;
        for _ in 0..WAIT_TIMEOUT {
            let in_report = self.recv()?;
            if let Some(reply) = in_report.subcmd_reply() {
                if reply.id() == subcmd.id() {
                    ensure!(reply.ack().is_ok(), "subcmd reply is nack");
                    return Ok(*reply);
                }
            }
        }

        bail!("Timeout while waiting for subcommand");
    }

    #[instrument(level = "info", skip(self), err)]
    pub fn read_spi<S: SPI>(&mut self) -> Result<S> {
        let reply = self.call_subcmd_wait(SPIReadRequest::new(S::range()))?;
        let result = reply.spi_read_result().unwrap();
        Ok((*result).try_into()?)
    }

    #[instrument(level = "info", skip(self), err)]
    pub fn read_spi_raw(&mut self, range: SPIRange) -> Result<[u8; 0x1D]> {
        let reply = self.call_subcmd_wait(SPIReadRequest::new(range))?;
        let result = reply.spi_read_result().unwrap();
        assert_eq!(result.range(), range);
        Ok(result.raw())
    }

    #[instrument(level = "info", skip(self), err)]
    pub fn write_spi<S: SPI + Into<SPIWriteRequest> + std::fmt::Debug>(
        &mut self,
        value: S,
    ) -> Result<bool> {
        let reply = self.call_subcmd_wait(value.into())?;
        Ok(reply.is_spi_write_success().unwrap())
    }

    #[instrument(level = "info", skip(self), err)]
    pub unsafe fn write_spi_raw(&mut self, range: SPIRange, data: &[u8]) -> Result<bool> {
        let reply = self.call_subcmd_wait(SPIWriteRequest::new(range, data))?;
        Ok(reply.is_spi_write_success().unwrap())
    }
}

/// MCU handling (infrared camera and NFC reader)
impl JoyCon {
    #[instrument(level = "info", skip(self), err)]
    pub fn enable_ir(&mut self, resolution: Resolution) -> Result<()> {
        self.enable_mcu()?;
        self.set_mcu_mode_ir()?;
        self.change_ir_resolution(resolution)?;
        Ok(())
    }

    #[instrument(level = "info", skip(self), err)]
    pub fn disable_mcu(&mut self) -> Result<()> {
        self.enable_ir_loop = false;
        self.set_report_mode_standard()?;
        self.call_subcmd_wait(SubcommandRequestEnum::SetMCUState(MCUMode::Suspend.into()))?;
        Ok(())
    }

    #[instrument(level = "info", skip(self), err)]
    pub fn enable_pulserate(&mut self) -> Result<()> {
        self.enable_mcu()?;
        self.set_mcu_mode_ir()?;
        self.set_ir_image_mode(MCUIRMode::PulseRate, 1)?;
        self.call_subcmd_wait(SubcommandRequestEnum::SetUnknownData([
            3, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 9,
        ]))?;
        Ok(())
    }

    #[instrument(level = "info", skip(self), err)]
    fn enable_mcu(&mut self) -> Result<()> {
        self.set_report_mode_mcu()?;
        self.call_subcmd_wait(SubcommandRequestEnum::SetMCUState(MCUMode::Standby.into()))?;
        self.wait_mcu_status(MCUMode::Standby)
            .context("enable_mcu")?;
        Ok(())
    }

    #[instrument(level = "info", skip(self), err)]
    fn set_report_mode_mcu(&mut self) -> Result<()> {
        self.call_subcmd_wait(SubcommandRequestEnum::SetInputReportMode(
            InputReportId::StandardFullMCU.into(),
        ))?;
        Ok(())
    }

    #[instrument(level = "info", skip(self), err)]
    fn set_mcu_mode_ir(&mut self) -> Result<()> {
        self.call_subcmd_wait(MCUCommand::set_mcu_mode(MCUMode::IR))?;
        self.wait_mcu_status(MCUMode::IR)
            .context("set_mcu_mode_ir")?;
        self.enable_ir_loop = true;
        Ok(())
    }

    #[instrument(level = "info", skip(self), err)]
    pub fn set_ir_image_mode(&mut self, ir_mode: MCUIRMode, frags: u8) -> Result<()> {
        let mut mcu_fw_version = Default::default();
        self.wait_mcu_cond(MCURequestEnum::GetMCUStatus(()), |r| {
            if let Some(status) = r.state_report() {
                mcu_fw_version = (status.fw_major_version, status.fw_minor_version);
                true
            } else {
                false
            }
        })?;
        let mcu_cmd = MCUCommand::configure_ir_ir(MCUIRModeData {
            ir_mode: ir_mode.into(),
            no_of_frags: frags,
            mcu_fw_version,
        });
        self.call_subcmd_wait(mcu_cmd)?;

        self.wait_mcu_cond(IRRequestEnum::GetState(()), |r| {
            r.ir_status()
                .map(|status| dbg!(status.ir_mode) == ir_mode)
                .unwrap_or(false)
        })
        .context("check sensor state")?;
        Ok(())
    }

    #[instrument(level = "info", skip(self), err)]
    pub fn get_ir_registers(&mut self) -> Result<Vec<Register>> {
        let mut registers = vec![];
        for page in 0..=4 {
            let offset = 0;
            let nb_registers = 0x6f;
            let request = IRRequest::from(IRReadRegisters {
                unknown_0x01: 0x01,
                page,
                offset,
                nb_registers,
            });
            let mcu_report = self
                .wait_mcu_cond(request, |mcu_report| {
                    if let Some(reg_slice) = mcu_report.ir_registers() {
                        reg_slice.page == page
                            && reg_slice.offset == offset
                            && reg_slice.nb_registers == nb_registers
                    } else {
                        false
                    }
                })
                .context("get IR registers slice")?;
            let reg_slice = mcu_report.ir_registers().expect("already validated above");
            registers.extend(Register::decode_raw(
                page,
                offset,
                &reg_slice.values[..reg_slice.nb_registers as usize],
            ));
        }
        Ok(registers)
    }

    #[instrument(level = "info", skip(self), err)]
    pub fn set_ir_registers(&mut self, regs: &[ir::Register]) -> Result<()> {
        let mut regs_mut = regs;
        while !regs_mut.is_empty() {
            let (mut report, remaining_regs) = OutputReport::set_registers(regs_mut);
            self.send(&mut report)?;
            regs_mut = remaining_regs;
            if !remaining_regs.is_empty() {
                // For packet drop purpose
                // TODO: not clean at all
                std::thread::sleep(std::time::Duration::from_millis(15));
            }
        }
        // TODO reg value doesn't change until next frame
        Ok(())
    }

    #[instrument(level = "info", skip(self), err)]
    pub fn change_ir_resolution(&mut self, resolution: Resolution) -> Result<()> {
        self.set_ir_wait_conf()
            .context("change_ir_resolution reset")?;
        self.set_ir_registers(&[Register::resolution(resolution), Register::finish()])
            .context("change_ir_resolution")?;
        self.set_ir_image_mode(MCUIRMode::ImageTransfer, resolution.max_fragment_id())
            .context("change_ir_resolution enable")?;
        #[cfg(feature = "ir")]
        self.image.change_resolution(resolution);
        Ok(())
    }

    #[instrument(level = "info", skip(self), err)]
    fn set_ir_wait_conf(&mut self) -> Result<()> {
        let mut mcu_fw_version = Default::default();
        self.wait_mcu_cond(MCURequestEnum::GetMCUStatus(()), |r| {
            if let Some(status) = r.state_report() {
                mcu_fw_version = (status.fw_major_version, status.fw_minor_version);
                true
            } else {
                false
            }
        })?;
        let mcu_cmd = MCUCommand::configure_ir_ir(MCUIRModeData {
            ir_mode: MCUIRMode::IRSensorReset.into(),
            no_of_frags: 0,
            mcu_fw_version,
        });
        self.call_subcmd_wait(mcu_cmd)?;

        self.wait_mcu_cond(IRRequestEnum::GetState(()), |r| {
            r.ir_status()
                .map(|status| status.ir_mode == MCUIRMode::WaitingForConfigurationMaybe)
                .unwrap_or(false)
        })
        .context("check sensor state")?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    fn send_mcu_subcmd(&mut self, mcu_subcmd: MCURequest) -> Result<()> {
        let mut out_report = mcu_subcmd.into();
        self.send(&mut out_report)?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self, f), err)]
    fn wait_mcu_cond<R: Into<MCURequest> + std::fmt::Debug>(
        &mut self,
        mcu_subcmd: R,
        mut f: impl FnMut(&MCUReport) -> bool,
    ) -> Result<MCUReport> {
        let mcu_subcmd = mcu_subcmd.into();
        // The MCU takes some time to warm up so we retry until we get an answer
        for _ in 0..WAIT_TIMEOUT {
            self.send_mcu_subcmd(mcu_subcmd)?;
            for _ in 0..WAIT_TIMEOUT {
                let in_report = self.recv()?;
                if let Some(mcu_report) = in_report.mcu_report() {
                    if f(mcu_report) {
                        return Ok(*mcu_report);
                    }
                }
            }
        }
        bail!("error getting the MCU status: timeout");
    }

    #[instrument(level = "debug", skip(self), err)]
    fn wait_mcu_status(&mut self, mode: MCUMode) -> Result<MCUReport> {
        self.wait_mcu_cond(MCURequestEnum::GetMCUStatus(()), |report| {
            report
                .state_report()
                .map(|status| status.state == mode)
                .unwrap_or(false)
        })
    }
}

/// IMU handling (gyroscope and accelerometer)
impl JoyCon {
    #[instrument(level = "info", skip(self), err)]
    pub fn enable_imu(&mut self) -> Result<()> {
        self.call_subcmd_wait(SubcommandRequestEnum::SetIMUMode(IMUMode::GyroAccel.into()))?;
        Ok(())
    }

    // TODO: needed?
    #[instrument(level = "info", skip(self), err)]
    pub fn set_imu_sens(&mut self) -> Result<()> {
        let gyro_sens = imu::GyroSens::DPS2000;
        let accel_sens = imu::AccSens::G8;
        self.call_subcmd_wait(imu::Sensitivity {
            gyro_sens: gyro_sens.into(),
            acc_sens: accel_sens.into(),
            ..imu::Sensitivity::default()
        })?;
        // TODO
        /*
        self.gyro_sens = gyro_sens;
        self.accel_sens = accel_sens;*/
        Ok(())
    }
}

/// Ringcon handling
impl JoyCon {
    #[instrument(level = "info", skip(self), err)]
    pub fn enable_ringcon(&mut self) -> Result<()> {
        self.call_subcmd_wait(SubcommandRequestEnum::SetMCUState(MCUMode::Standby.into()))?;
        loop {
            let out = self.call_subcmd_wait(MCUCommand::set_mcu_mode(MCUMode::MaybeRingcon))?;
            if out.mcu_report().unwrap().state_report().unwrap().state == MCUMode::MaybeRingcon {
                break;
            }
        }
        self.call_subcmd_wait(MCUCommand::configure_mcu_ir(MCUIRModeData {
            ir_mode: MCUIRMode::IRSensorSleep.into(),
            no_of_frags: 0,
            mcu_fw_version: (0.into(), 0.into()),
        }))?;
        self.call_subcmd_wait(SubcommandRequest::subcmd_0x59())?;
        self.call_subcmd_wait(SubcommandRequestEnum::SetIMUMode(
            IMUMode::MaybeRingcon.into(),
        ))?;
        self.call_subcmd_wait(SubcommandRequest::subcmd_0x5c_6())?;
        self.call_subcmd_wait(SubcommandRequest::subcmd_0x5a())?;
        Ok(())
    }

    #[instrument(level = "info", skip(self), err)]
    pub fn disable_ringcon(&mut self) -> Result<()> {
        self.call_subcmd_wait(SubcommandRequest::subcmd_0x5b())?;
        self.call_subcmd_wait(SubcommandRequestEnum::SetIMUMode(
            IMUMode::_Unknown0x02.into(),
        ))?;
        self.call_subcmd_wait(SubcommandRequest::subcmd_0x5c_0())?;
        self.call_subcmd_wait(MCUCommand::configure_mcu_ir(MCUIRModeData {
            ir_mode: MCUIRMode::IRSensorReset.into(),
            no_of_frags: 0,
            mcu_fw_version: (0.into(), 0.into()),
        }))?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    pub fn mcu_wait_not_busy(&mut self) -> anyhow::Result<()> {
        loop {
            let report = self.recv()?;
            if let Some(x) = report.mcu_report() {
                if x.id() != MCUReportId::BusyInitializing {
                    return Ok(());
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
