use crate::image::Image;
use crate::{imu_handler, Position};
use anyhow::{bail, ensure, Context, Result};
use joycon_sys::input::*;
use joycon_sys::light;
use joycon_sys::mcu::ir::*;
use joycon_sys::mcu::*;
use joycon_sys::output::*;
use joycon_sys::spi::*;
use joycon_sys::*;

const WAIT_TIMEOUT: u32 = 60;

#[derive(Debug, Clone)]
pub struct Report {
    pub left_stick: (f64, f64),
    pub right_stick: (f64, f64),
    pub info: DeviceStatus,
    pub image: Option<image::GrayImage>,
    pub position: Position,
}

pub struct JoyCon {
    device: hidapi::HidDevice,
    info: hidapi::DeviceInfo,
    counter: u8,
    pub max_raw_gyro: i16,
    pub max_raw_accel: i16,
    left_stick_calib: StickCalibration,
    right_stick_calib: StickCalibration,
    image: Image,
    enable_ir_loop: bool,
    imu_handler: crate::imu_handler::Handler,
}

impl JoyCon {
    pub fn new(device: hidapi::HidDevice, info: hidapi::DeviceInfo) -> Result<JoyCon> {
        assert!([
            JOYCON_L_BT,
            JOYCON_R_BT,
            PRO_CONTROLLER,
            JOYCON_CHARGING_GRIP,
        ]
        .contains(&info.product_id()));
        let mut joycon = JoyCon {
            device,
            info,
            counter: 0,
            max_raw_gyro: 0,
            max_raw_accel: 0,
            left_stick_calib: StickCalibration::default(),
            right_stick_calib: StickCalibration::default(),
            image: Image::new(),
            enable_ir_loop: false,
            imu_handler: crate::imu_handler::Handler::new(
                imu::GyroSens::default(),
                imu::AccSens::default(),
            ),
        };

        joycon.set_report_mode_standard()?;
        Ok(joycon)
    }

    fn send(&mut self, report: &mut OutputReport) -> Result<()> {
        report.packet_counter = self.counter;
        self.counter = (self.counter + 1) & 0xf;
        let buffer = report.as_bytes();
        let nb_written = self.device.write(buffer)?;
        assert_eq!(nb_written, buffer.len());
        Ok(())
    }

    fn recv(&mut self) -> Result<InputReport> {
        // Larger buffer to detect unhandled received data
        let mut reports = [InputReport::new(); 2];
        let buffer = unsafe {
            std::slice::from_raw_parts_mut(
                &mut reports as *mut _ as *mut u8,
                std::mem::size_of::<InputReport>(),
            )
        };
        let nb_read = self.device.read(buffer)?;
        let report = reports[0];
        report.validate();
        assert_eq!(nb_read, std::mem::size_of_val(&report));
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
            info: std_report.info,
            image: self.image.last_image.take(),
            position: *self.imu_handler.position(),
        })
    }

    pub fn load_calibration(&mut self) -> Result<()> {
        let factory_result = self.read_spi(RANGE_FACTORY_CALIBRATION_SENSORS)?;
        let factory_settings = factory_result.imu_factory_calib().unwrap();
        self.imu_handler.set_factory(*factory_settings);

        let user_result = self.read_spi(RANGE_USER_CALIBRATION_SENSORS)?;
        let user_settings = user_result.imu_user_calib().unwrap();
        self.imu_handler.set_user(*user_settings);

        self.imu_handler.reset_calibration();

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

    fn read_spi(&mut self, range: SPIRange) -> Result<SPIReadResult> {
        let reply = self.send_subcmd_wait(SPIReadRequest::new(range))?;
        let result = reply.spi_result().unwrap();
        ensure!(
            range == result.range(),
            "invalid range {:?}",
            result.range()
        );
        Ok(*result)
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
        let mcu_cmd = MCUCommand::configure_ir(MCUIRModeData {
            ir_mode,
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
        let mcu_cmd = MCUCommand::configure_ir(MCUIRModeData {
            ir_mode: MCUIRMode::IRSensorReset,
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
                .map(|status| status.state == mode)
                .unwrap_or(false)
        })
    }
}

/// IMU handling (gyroscope and accelerometer)
impl JoyCon {
    pub fn enable_imu(&mut self) -> Result<()> {
        self.send_subcmd_wait(SubcommandRequest::set_imu_enabled(true))?;
        Ok(())
    }

    pub fn set_imu_callback(&mut self, cb: Box<dyn FnMut(&imu_handler::Position)>) {
        self.imu_handler.set_cb(cb);
    }

    // TODO: needed?
    pub fn set_imu_sens(&mut self) -> Result<()> {
        let gyro_sens = imu::GyroSens::DPS2000;
        let accel_sens = imu::AccSens::G8;
        self.send_subcmd_wait(imu::Sensitivity {
            gyro_sens,
            acc_sens: accel_sens,
            ..imu::Sensitivity::default()
        })?;
        // TODO
        /*
        self.gyro_sens = gyro_sens;
        self.accel_sens = accel_sens;*/
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
