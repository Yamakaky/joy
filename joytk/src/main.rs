use anyhow::{bail, Result};
use cgmath::{Deg, Euler, One, Quaternion, Vector3};
use clap::Clap;
use joycon::{
    hidapi::HidApi,
    joycon_sys::{
        accessory::AccessoryCommand,
        input::{BatteryLevel, Stick, UseSPIColors, WhichController},
        light::{self, PlayerLight},
        spi::{
            ControllerColor, SPIRange, SensorCalibration, SticksCalibration, UserSensorCalibration,
            UserSticksCalibration,
        },
        InputReport, OutputReport, NINTENDO_VENDOR_ID,
    },
    JoyCon,
};
use std::{
    fs::OpenOptions,
    io::{BufRead, Write},
    time::Duration,
};
use std::{thread::sleep, time::Instant};

#[cfg(target_os = "linux")]
mod relay;

/// Access every feature of the Nintendo Switch controllers
#[derive(Clap)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
    /// Wait for a controller to connect
    #[clap(short, long)]
    wait: bool,
}

#[derive(Clap)]
enum SubCommand {
    /// Calibrate the controller
    ///
    /// The calibration will be stored on the controller and used by the Switch.
    Calibrate(Calibrate),
    /// Print settings from the controller
    Get,
    /// Configure settings of the controller
    Set(Set),
    /// Show live inputs from the controller
    Monitor,
    /// Dump the memory of the controller to a binary file
    Dump,
    /// Restore the memory of the controller from a dump file
    Restore,
    /// Decode raw reports exchanged between the controller and the Switch
    ///
    /// See the `trace/` folder for recorded dumps, and
    /// [relay_joycon.py](https://github.com/Yamakaky/joycontrol/blob/capture-text-file/scripts/relay_joycon.py)
    /// for capturing new dumps.
    Decode,
    Relay,
    /// Ringcon-specific actions
    Ringcon(Ringcon),
}

#[derive(Clap)]
struct Calibrate {
    #[clap(subcommand)]
    subcmd: CalibrateE,
}

#[derive(Clap)]
enum CalibrateE {
    /// Calibrate the sticks
    Sticks,
    /// Calibrate the gyroscope
    Gyroscope,
    /// Reset gyroscope and sticks calibration to factory values
    Reset,
}

#[derive(Clap)]
struct Set {
    #[clap(subcommand)]
    subcmd: SetE,
}

#[derive(Clap)]
enum SetE {
    /// Change the color of the controller
    ///
    /// This is used by the switch for the controller icons. Every color is in `RRGGBB` format.
    Color(SetColor),
}

#[derive(Clap)]
struct SetColor {
    /// Color of the body of the controller
    body: String,
    /// Color of the buttons, sticks and triggers
    buttons: String,
    /// Color of the left grip (Pro Controller only)
    left_grip: Option<String>,
    /// Color of the right grip (Pro Controller only)
    right_grip: Option<String>,
}

#[derive(Clap)]
struct Ringcon {
    #[clap(subcommand)]
    subcmd: RingconE,
}

#[derive(Clap)]
enum RingconE {
    /// Get the number of flex stored in the ringcon
    StoredFlex,
    /// Show the flex value in realtime
    Monitor,
    /// Random experiments
    Exp,
}

fn main() -> Result<()> {
    let opts = Opts::parse();

    if let SubCommand::Decode = opts.subcmd {
        return decode();
    }

    let api = HidApi::new()?;
    loop {
        if let Some(device_info) = api
            .device_list()
            .find(|x| x.vendor_id() == NINTENDO_VENDOR_ID)
        {
            let device = device_info.open_device(&api)?;

            if let SubCommand::Relay = opts.subcmd {
                if cfg!(target_os = "linux") {
                    relay::relay(device)?;
                } else {
                    bail!("relaying only works on linux");
                }
            } else {
                let joycon = JoyCon::new(device, device_info.clone())?;

                hid_main(joycon, &opts)?;
            }

            break;
        } else if !opts.wait {
            eprintln!("No device found");
            break;
        } else {
            sleep(Duration::from_millis(200));
        }
    }
    Ok(())
}

fn hid_main(mut joycon: JoyCon, opts: &Opts) -> Result<()> {
    joycon.set_home_light(light::HomeLight::new(
        0x8,
        0x2,
        0x0,
        &[(0xf, 0xf, 0), (0x2, 0xf, 0)],
    ))?;

    let battery_level = joycon.tick()?.info.battery_level();

    joycon.set_player_light(light::PlayerLights::new(
        (battery_level >= BatteryLevel::Full).into(),
        (battery_level >= BatteryLevel::Medium).into(),
        (battery_level >= BatteryLevel::Low).into(),
        if battery_level >= BatteryLevel::Low {
            PlayerLight::On
        } else {
            PlayerLight::Blinking
        },
    ))?;

    match opts.subcmd {
        SubCommand::Calibrate(ref calib) => match calib.subcmd {
            CalibrateE::Sticks => calibrate_sticks(&mut joycon)?,
            CalibrateE::Gyroscope => calibrate_gyro(&mut joycon)?,
            CalibrateE::Reset => reset_calibration(&mut joycon)?,
        },
        SubCommand::Get => get(&mut joycon)?,
        SubCommand::Set(ref set) => match set.subcmd {
            SetE::Color(ref arg) => set_color(&mut joycon, arg)?,
        },
        SubCommand::Monitor => monitor(&mut joycon)?,
        SubCommand::Dump => dump(&mut joycon)?,
        SubCommand::Restore => restore(&mut joycon)?,
        SubCommand::Ringcon(ref cmd) => ringcon(&mut joycon, cmd)?,
        SubCommand::Decode | SubCommand::Relay => unreachable!(),
    }
    Ok(())
}

fn reset_calibration(joycon: &mut JoyCon) -> Result<()> {
    joycon.write_spi(UserSensorCalibration::reset())?;
    // TODO: reset sticks
    Ok(())
}

fn dump(joycon: &mut JoyCon) -> Result<()> {
    let mut out = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open("/tmp/out.raw")?;
    let mut offset = 0;
    let size = 0x1D;
    // TODO: why value
    let mut last_percent = 0;
    while offset + (size as u32) < 0x80000 {
        let percent = offset * 100 / 0x80000;
        if last_percent != percent {
            println!("{}%", percent);
            last_percent = percent;
        }
        let slice = joycon.read_spi_raw(unsafe { SPIRange::new(offset, size) })?;
        out.write(&slice)?;
        offset += size as u32;
    }
    let last_size = (0x80000 - offset) as u8;
    let slice = joycon.read_spi_raw(unsafe { SPIRange::new(offset, last_size) })?;
    out.write(&slice)?;
    Ok(())
}

fn restore(_joycon: &mut JoyCon) -> Result<()> {
    unimplemented!()
}

fn calibrate_gyro(joycon: &mut JoyCon) -> Result<()> {
    joycon.enable_imu()?;
    println!("Don't move the controller...");
    sleep(Duration::from_secs(1));

    let mut gyro_reports = Vec::new();
    let mut acc_reports = Vec::new();
    for i in (0..10).rev() {
        print!("{}, ", i);
        std::io::stdout().flush()?;
        let now = Instant::now();
        while now.elapsed() < Duration::from_secs(1) {
            let report = joycon.tick()?;
            gyro_reports.extend(
                report
                    .raw
                    .imu_frames()
                    .unwrap()
                    .iter()
                    .map(|x| x.raw_gyro()),
            );
            acc_reports.extend(
                report
                    .raw
                    .imu_frames()
                    .unwrap()
                    .iter()
                    .map(|x| x.raw_accel()),
            );
        }
    }
    println!();
    let gyro_avg = gyro_reports.iter().sum::<Vector3<f64>>() / gyro_reports.len() as f64;

    let factory: SensorCalibration = joycon.read_spi()?;
    let user: UserSensorCalibration = joycon.read_spi()?;
    let mut calib = user.calib().unwrap_or(factory);
    calib.set_gyro_offset(gyro_avg);

    println!("Writing calibration data {:x?}", calib);
    joycon.write_spi(UserSensorCalibration::from(calib))?;

    Ok(())
}

fn calibrate_sticks(joycon: &mut JoyCon) -> Result<()> {
    println!("Don't move the sticks...");
    sleep(Duration::from_secs(1));
    let (left_neutral, right_neutral) = raw_sticks(joycon)?;

    println!("Move the sticks then press A...");
    let mut l_x_min = left_neutral.x();
    let mut l_x_max = left_neutral.x();
    let mut l_y_min = left_neutral.y();
    let mut l_y_max = left_neutral.y();
    let mut r_x_min = right_neutral.x();
    let mut r_x_max = right_neutral.x();
    let mut r_y_min = right_neutral.y();
    let mut r_y_max = right_neutral.y();

    loop {
        let report = joycon.tick()?;
        let (left_stick, right_stick) = raw_sticks(joycon)?;

        if report.buttons.right.a() {
            break;
        }

        l_x_min = l_x_min.min(left_stick.x());
        l_x_max = l_x_max.max(left_stick.x());
        l_y_min = l_y_min.min(left_stick.y());
        l_y_max = l_y_max.max(left_stick.y());
        r_x_min = r_x_min.min(right_stick.x());
        r_x_max = r_x_max.max(right_stick.x());
        r_y_min = r_y_min.min(right_stick.y());
        r_y_max = r_y_max.max(right_stick.y());
    }

    dbg!((l_x_min, left_neutral.x(), l_x_max));
    dbg!((l_y_min, left_neutral.y(), l_y_max));
    // TODO: write calibration

    Ok(())
}

fn raw_sticks(joycon: &mut JoyCon) -> Result<(Stick, Stick)> {
    let report = joycon.recv()?;
    let std_report = report.standard().expect("should be standard");
    Ok((std_report.left_stick, std_report.right_stick))
}

fn get(joycon: &mut JoyCon) -> Result<()> {
    let dev_info = joycon.get_dev_info()?;
    println!(
        "{}, MAC {}, firmware version {}",
        dev_info.which_controller, dev_info.mac_address, dev_info.firmware_version
    );
    println!();

    println!("Controller color:");
    let color: ControllerColor = joycon.read_spi()?;
    println!("  body: {}", color.body);
    println!("  buttons: {}", color.buttons);
    if dev_info.use_spi_colors == UseSPIColors::IncludingGrip {
        println!("  left grip: {}", color.left_grip);
        println!("  right grip: {}", color.right_grip);
    }
    println!();

    let imu_factory_settings: SensorCalibration = joycon.read_spi()?;
    let imu_user_settings: UserSensorCalibration = joycon.read_spi()?;

    println!("Gyroscope calibration data:");
    println!(
        "  factory: offset ({:?}), factor ({:?})",
        imu_factory_settings.gyro_offset().cast::<i16>().unwrap(),
        imu_factory_settings.gyro_factor().cast::<u16>().unwrap(),
    );
    if let Some(settings) = imu_user_settings.calib() {
        println!(
            "  user: offset ({:?}), factor ({:?})",
            settings.gyro_offset().cast::<i16>().unwrap(),
            settings.gyro_factor().cast::<u16>().unwrap(),
        );
    } else {
        println!("  no user");
    }
    println!("");
    println!("Accelerometer calibration data:");
    println!(
        "  factory: offset ({:?}), factor ({:?})",
        imu_factory_settings.acc_offset().cast::<i16>().unwrap(),
        imu_factory_settings.acc_factor().cast::<u16>().unwrap(),
    );
    if let Some(settings) = imu_user_settings.calib() {
        println!(
            "  user: offset ({:?}), factor ({:?})",
            settings.acc_offset().cast::<i16>().unwrap(),
            settings.acc_factor().cast::<u16>().unwrap(),
        );
    } else {
        println!("  no user");
    }
    println!("");

    let sticks_factory_settings: SticksCalibration = joycon.read_spi()?;
    let sticks_user_settings: UserSticksCalibration = joycon.read_spi()?;
    println!("Left stick calibration data");
    println!(
        "  factory: min {:x?}, center {:x?}, max {:x?}",
        sticks_factory_settings.left.min(),
        sticks_factory_settings.left.center(),
        sticks_factory_settings.left.max()
    );
    if let Some(left) = sticks_user_settings.left.calib() {
        println!(
            "  user: min {:x?}, center {:x?}, max {:x?}",
            left.min(),
            left.center(),
            left.max()
        );
    } else {
        println!("  no user");
    }
    println!("");
    println!("Right stick calibration data");
    println!(
        "  factory: min {:x?}, center {:x?}, max {:x?}",
        sticks_factory_settings.right.min(),
        sticks_factory_settings.right.center(),
        sticks_factory_settings.right.max()
    );
    if let Some(right) = sticks_user_settings.right.calib() {
        println!(
            "  user: min {:x?}, center {:x?}, max {:x?}",
            right.min(),
            right.center(),
            right.max()
        );
    } else {
        println!("  no user");
    }
    println!("");

    Ok(())
}

fn set_color(joycon: &mut JoyCon, arg: &SetColor) -> Result<()> {
    let dev_info = joycon.get_dev_info()?;
    let is_pro_controller = dev_info.which_controller == WhichController::ProController;

    let mut colors = ControllerColor {
        body: arg.body.parse()?,
        buttons: arg.buttons.parse()?,
        ..Default::default()
    };
    if let (Some(left), Some(right)) = (arg.left_grip.as_ref(), arg.right_grip.as_ref()) {
        if is_pro_controller {
            colors.left_grip = left.parse()?;
            colors.right_grip = right.parse()?;
            if dev_info.use_spi_colors != UseSPIColors::IncludingGrip {
                joycon.write_spi(UseSPIColors::IncludingGrip)?;
            }
        } else {
            panic!("grips can only be set on pro controller");
        }
    }
    println!("Setting controller colors to {:x?}", colors);
    joycon.write_spi(colors)?;
    println!("Reconnect your controller");
    Ok(())
}

fn monitor(joycon: &mut JoyCon) -> Result<()> {
    joycon.enable_imu()?;
    joycon.load_calibration()?;
    let mut orientation = Quaternion::one();
    let mut now = Instant::now();
    loop {
        let report = joycon.tick()?;
        let mut last_acc = Vector3::unit_x();
        let mut last_rot = Vector3::unit_x();
        for frame in &report.imu.unwrap() {
            orientation = orientation
                * Quaternion::from(Euler::new(
                    Deg(frame.gyro.y * 0.005),
                    Deg(frame.gyro.z * 0.005),
                    Deg(frame.gyro.x * 0.005),
                ));
            last_acc = frame.accel;
            last_rot = frame.gyro;
        }
        if now.elapsed() > Duration::from_millis(100) {
            now = Instant::now();
            println!("Clicked: {}", report.buttons);

            let euler_rot = Euler::from(orientation);
            let pitch = Deg::from(euler_rot.x);
            let yaw = Deg::from(euler_rot.y);
            let roll = Deg::from(euler_rot.z);
            println!(
                "Rotation: pitch {:?}, yaw {:?}, roll {:?}",
                pitch, yaw, roll
            );
            println!("Rotation speed: {:#?}", last_rot);
            println!("Acceleration: {:?}", last_acc);
        }
    }
}

fn decode() -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let mut image = joycon::Image::new();
    image.change_resolution(joycon::joycon_sys::mcu::ir::Resolution::R320x240);
    for line in stdin.lock().lines() {
        let line = line?;
        let fragments: Vec<&str> = line.split(" ").collect();
        let side = fragments[0];
        let time = fragments[1];
        let hex = hex::decode(&fragments[2][2..])?;
        if side == ">" {
            let mut report = InputReport::new();
            let raw_report = report.as_bytes_mut();
            let len = raw_report.len().min(hex.len());
            raw_report[..len].copy_from_slice(&hex[..len]);
            if let Some(subcmd) = report.subcmd_reply() {
                println!("{} {:?}", time, subcmd);
            } else if let Some(mcu) = report.mcu_report() {
                println!("{} {:?}", time, mcu);
                image.handle(mcu);
                if let Some(img) = image.last_image.take() {
                    img.save("/tmp/out.png")?;
                    dbg!("new image");
                }
            }
        } else {
            let mut report = OutputReport::new();
            let raw_report = report.as_bytes_mut();
            let len = raw_report.len().min(hex.len());
            raw_report[..len].copy_from_slice(&hex[..len]);
            if let Some(subcmd) = report.subcmd_request() {
                println!("{} {:?}", time, subcmd);
            } else if let Some(mcu) = report.mcu_request() {
                println!("{} {:?}", time, mcu);
            }
        }
    }

    Ok(())
}

fn ringcon(joycon: &mut JoyCon, cmd: &Ringcon) -> anyhow::Result<()> {
    println!("Ringcon initialisation...");
    joycon.enable_ringcon()?;
    match cmd.subcmd {
        RingconE::StoredFlex => {
            println!(
                "Stored steps: {}",
                joycon
                    .call_subcmd_wait(AccessoryCommand::get_offline_steps())?
                    .accessory_response()
                    .unwrap()
                    .offline_steps()?
                    .steps
            );
        }
        RingconE::Monitor => loop {
            let report = joycon.recv()?;
            let frames = report.imu_frames().unwrap();
            println!("Flex value: {}", frames[2].raw_ringcon());
        },
        RingconE::Exp => {}
    }
    Ok(())
}
