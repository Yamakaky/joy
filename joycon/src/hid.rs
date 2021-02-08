use std::convert::TryInto;

use crate::image::Image;
use crate::imu_handler;
use anyhow::{bail, ensure, Context, Result};
use cgmath::Vector2;
use joycon_sys::mcu::*;
use joycon_sys::output::*;
use joycon_sys::spi::*;
use joycon_sys::*;
use joycon_sys::{imu::IMUMode, mcu::ir::*};
use joycon_sys::{input::*, light};

const WAIT_TIMEOUT: u32 = 200;

#[derive(Debug, Clone)]
pub struct Report {
    pub left_stick: Vector2<f64>,
    pub right_stick: Vector2<f64>,
    pub buttons: ButtonsStatus,
    pub info: DeviceStatus,
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
    image: Image,
    enable_ir_loop: bool,
    imu_handler: crate::imu_handler::Handler,
    device_type: WhichController,
}

impl JoyCon {
    pub fn new(device: hidapi::HidDevice, info: hidapi::DeviceInfo) -> Result<JoyCon> {
        let device_type = match info.product_id() {
            JOYCON_L_BT => WhichController::LeftJoyCon,
            JOYCON_R_BT => WhichController::RightJoyCon,
            PRO_CONTROLLER => WhichController::ProController,
            JOYCON_CHARGING_GRIP | _ => panic!("unknown controller type"),
        };
        let mut joycon = JoyCon {
            device,
            info,
            counter: 0,
            max_raw_gyro: 0,
            max_raw_accel: 0,
            left_stick_calib: LeftStickCalibration::default(),
            right_stick_calib: RightStickCalibration::default(),
            image: Image::new(),
            enable_ir_loop: false,
            imu_handler: crate::imu_handler::Handler::new(
                device_type,
                imu::GyroSens::default(),
                imu::AccSens::default(),
            ),
            device_type,
        };

        joycon.send_subcmd_wait(SubcommandRequest::disable_shipment_mode())?;
        joycon.set_report_mode_standard()?;
        Ok(joycon)
    }

    pub fn supports_ir(&self) -> bool {
        self.device_type == WhichController::RightJoyCon
    }

    pub fn send(&mut self, report: &mut OutputReport) -> Result<()> {
        report.packet_counter = self.counter;
        self.counter = (self.counter + 1) & 0xf;
        let buffer = report.as_bytes();
        let nb_written = self.device.write(buffer)?;
        assert_eq!(nb_written, buffer.len());
        Ok(())
    }

    pub fn recv(&mut self) -> Result<InputReport> {
        let mut report = InputReport::new();
        let buffer = report.as_bytes_mut();
        // TODO: 64 byte on pro controller, why ?
        let _nb_read = self.device.read(buffer)?;
        //dbg!(nb_read);
        //assert_eq!(nb_read, buffer.len());
        report.validate();
        if let Some(frames) = report.imu_frames() {
            self.imu_handler.handle_frames(frames);
        }
        if let Some(mcu_report) = report.mcu_report() {
            if self.enable_ir_loop {
                for packet in &mut self.image.handle(&mcu_report) {
                    if let Some(mut packet) = packet {
                        self.send(&mut packet)?;
                    }
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

    pub fn get_dev_info(&mut self) -> Result<DeviceInfo> {
        let reply = self.send_subcmd_wait(SubcommandRequest::request_device_info())?;
        Ok(*reply.device_info().unwrap())
    }

    pub fn set_home_light(&mut self, home_light: light::HomeLight) -> Result<()> {
        self.send_subcmd_wait(home_light)?;
        Ok(())
    }

    pub fn set_player_light(&mut self, player_lights: light::PlayerLights) -> Result<()> {
        self.send_subcmd_wait(player_lights)?;
        Ok(())
    }

    fn set_report_mode_standard(&mut self) -> Result<()> {
        self.send_subcmd_wait(SubcommandRequest::set_input_report_mode(
            InputReportId::StandardFull,
        ))?;
        Ok(())
    }

    fn send_subcmd_wait<S: Into<SubcommandRequest>>(
        &mut self,
        subcmd: S,
    ) -> Result<SubcommandReply> {
        let subcmd = subcmd.into();
        let mut out_report = subcmd.into();

        self.send(&mut out_report)?;
        for _ in 0..WAIT_TIMEOUT {
            let in_report = self.recv()?;
            if let Some(reply) = in_report.subcmd_reply() {
                if reply.id() == Some(subcmd.id()) {
                    ensure!(reply.ack.is_ok(), "subcmd reply is nack");
                    return Ok(*reply);
                }
            }
        }

        bail!("Timeout while waiting for subcommand");
    }

    pub fn read_spi<S: SPI>(&mut self) -> Result<S> {
        let reply = self.send_subcmd_wait(SPIReadRequest::new(S::range()))?;
        let result = reply.spi_result().unwrap();
        Ok((*result).try_into()?)
    }

    pub fn read_spi_raw(&mut self, range: SPIRange) -> Result<[u8; 0x1D]> {
        let reply = self.send_subcmd_wait(SPIReadRequest::new(range))?;
        let result = reply.spi_result().unwrap();
        assert_eq!(result.range(), range);
        Ok(result.raw())
    }

    pub fn write_spi<S: SPI + Into<SPIWriteRequest>>(&mut self, value: S) -> Result<bool> {
        let reply = self.send_subcmd_wait(value.into())?;
        Ok(reply.spi_write_success().unwrap())
    }

    pub unsafe fn write_spi_raw(&mut self, range: SPIRange, data: &[u8]) -> Result<bool> {
        let reply = self.send_subcmd_wait(SPIWriteRequest::new(range, data))?;
        Ok(reply.spi_write_success().unwrap())
    }
}

/// MCU handling (infrared camera and NFC reader)
impl JoyCon {
    pub fn enable_ir(&mut self, resolution: Resolution) -> Result<()> {
        self.enable_mcu()?;
        self.set_mcu_mode_ir()?;
        self.change_ir_resolution(resolution)?;
        Ok(())
    }

    pub fn disable_mcu(&mut self) -> Result<()> {
        self.enable_ir_loop = false;
        self.set_report_mode_standard()?;
        self.send_subcmd_wait(SubcommandRequest::set_mcu_mode(MCUMode::Suspend))?;
        Ok(())
    }

    fn enable_mcu(&mut self) -> Result<()> {
        self.set_report_mode_mcu()?;
        self.send_subcmd_wait(SubcommandRequest::set_mcu_mode(MCUMode::Standby))?;
        self.wait_mcu_status(MCUMode::Standby)
            .context("enable_mcu")?;
        Ok(())
    }

    fn set_report_mode_mcu(&mut self) -> Result<()> {
        self.send_subcmd_wait(SubcommandRequest::set_input_report_mode(
            InputReportId::StandardFullMCU,
        ))?;
        Ok(())
    }

    fn set_mcu_mode_ir(&mut self) -> Result<()> {
        self.send_subcmd_wait(MCUCommand::set_mcu_mode(MCUMode::IR))?;
        self.wait_mcu_status(MCUMode::IR)
            .context("set_mcu_mode_ir")?;
        self.enable_ir_loop = true;
        Ok(())
    }

    fn set_ir_image_mode(&mut self, ir_mode: MCUIRMode, frags: u8) -> Result<()> {
        let mut mcu_fw_version = Default::default();
        self.wait_mcu_cond(MCURequest::get_mcu_status(), |r| {
            if let Some(status) = r.as_status() {
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
        self.send_subcmd_wait(mcu_cmd)?;

        self.wait_mcu_cond(IRRequest::get_state(), |r| {
            r.as_ir_status()
                .map(|status| dbg!(status.ir_mode) == ir_mode)
                .unwrap_or(false)
        })
        .context("check sensor state")?;
        Ok(())
    }

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
                    if let Some(reg_slice) = mcu_report.as_ir_registers() {
                        reg_slice.page == page
                            && reg_slice.offset == offset
                            && reg_slice.nb_registers == nb_registers
                    } else {
                        false
                    }
                })
                .context("get IR registers slice")?;
            let reg_slice = mcu_report
                .as_ir_registers()
                .expect("already validated above");
            registers.extend(Register::decode_raw(
                page,
                offset,
                &reg_slice.values[..reg_slice.nb_registers as usize],
            ));
        }
        Ok(registers)
    }

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

    pub fn change_ir_resolution(&mut self, resolution: Resolution) -> Result<()> {
        self.set_ir_wait_conf()
            .context("change_ir_resolution reset")?;
        self.set_ir_registers(&[Register::resolution(resolution), Register::finish()])
            .context("change_ir_resolution")?;
        self.set_ir_image_mode(MCUIRMode::ImageTransfer, resolution.max_fragment_id())
            .context("change_ir_resolution enable")?;
        self.image.change_resolution(resolution);
        Ok(())
    }

    fn set_ir_wait_conf(&mut self) -> Result<()> {
        let mut mcu_fw_version = Default::default();
        self.wait_mcu_cond(MCURequest::get_mcu_status(), |r| {
            if let Some(status) = r.as_status() {
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
        self.send_subcmd_wait(mcu_cmd)?;

        self.wait_mcu_cond(IRRequest::get_state(), |r| {
            r.as_ir_status()
                .map(|status| status.ir_mode == MCUIRMode::WaitingForConfigurationMaybe)
                .unwrap_or(false)
        })
        .context("check sensor state")?;
        Ok(())
    }

    fn send_mcu_subcmd(&mut self, mcu_subcmd: MCURequest) -> Result<()> {
        let mut out_report = mcu_subcmd.into();
        self.send(&mut out_report)?;
        Ok(())
    }

    fn wait_mcu_cond<R: Into<MCURequest>>(
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

    fn wait_mcu_status(&mut self, mode: MCUMode) -> Result<MCUReport> {
        self.wait_mcu_cond(MCURequest::get_mcu_status(), |report| {
            report
                .as_status()
                .map(|status| dbg!(status.state) == mode)
                .unwrap_or(false)
        })
    }
}

/// IMU handling (gyroscope and accelerometer)
impl JoyCon {
    pub fn enable_imu(&mut self) -> Result<()> {
        self.send_subcmd_wait(SubcommandRequest::set_imu_mode(IMUMode::GyroAccel))?;
        Ok(())
    }

    // TODO: needed?
    pub fn set_imu_sens(&mut self) -> Result<()> {
        let gyro_sens = imu::GyroSens::DPS2000;
        let accel_sens = imu::AccSens::G8;
        self.send_subcmd_wait(imu::Sensitivity {
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
    pub fn enable_ringcon(&mut self) -> Result<()> {
        self.send_subcmd_wait(SubcommandRequest::set_imu_mode(IMUMode::_Unknown0x02))?;
        self.send_subcmd_wait(SubcommandRequest::set_mcu_mode(MCUMode::Standby))?;
        loop {
            let out = self.send_subcmd_wait(MCUCommand::set_mcu_mode(MCUMode::Suspend))?;
            if out.mcu_report().unwrap().as_status().unwrap().state == MCUMode::Standby {
                break;
            }
        }
        self.send_subcmd_wait(SubcommandRequest::set_mcu_mode(MCUMode::Standby))?;
        loop {
            let out = self.send_subcmd_wait(MCUCommand::set_mcu_mode(MCUMode::MaybeRingcon))?;
            if out.mcu_report().unwrap().as_status().unwrap().state == MCUMode::MaybeRingcon {
                break;
            }
        }
        self.send_subcmd_wait(MCUCommand::configure_mcu_ir(MCUIRModeData {
            ir_mode: MCUIRMode::IRSensorSleep.into(),
            no_of_frags: 0,
            mcu_fw_version: (0.into(), 0.into()),
        }))?;
        self.send_subcmd_wait(SubcommandRequest::subcmd_0x59())?;
        self.send_subcmd_wait(SubcommandRequest::set_imu_mode(IMUMode::MaybeRingcon))?;
        self.send_subcmd_wait(SubcommandRequest::subcmd_0x5c())?;
        self.send_subcmd_wait(SubcommandRequest::subcmd_0x5a())?;
        Ok(())
    }

    pub fn mcu_wait_not_busy(&mut self) -> anyhow::Result<()> {
        loop {
            let report = self.recv()?;
            if let Some(x) = report.mcu_report() {
                dbg!(x.id);
                if x.id != MCUReportId::BusyInitializing {
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
