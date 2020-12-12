use anyhow::Result;
use cgmath::{Deg, Euler, One, Quaternion};
use clap::Clap;
use joycon::{
    hidapi::HidApi,
    joycon_sys::{
        input::{BatteryLevel, Stick, UseSPIColors, WhichController},
        light::{self, PlayerLight},
        spi::{
            ControllerColor, RANGE_CONTROLLER_COLOR, RANGE_FACTORY_CALIBRATION_SENSORS,
            RANGE_FACTORY_CALIBRATION_STICKS, RANGE_USER_CALIBRATION_SENSORS,
            RANGE_USER_CALIBRATION_STICKS,
        },
        NINTENDO_VENDOR_ID,
    },
    JoyCon,
};
use std::time::Duration;
use std::{thread::sleep, time::Instant};

#[derive(Clap)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    Calibrate(Calibrate),
    Get,
    Set(Set),
    Monitor,
}

#[derive(Clap)]
struct Calibrate {
    #[clap(subcommand)]
    subcmd: CalibrateE,
}

#[derive(Clap)]
enum CalibrateE {
    Sticks,
    Gyroscope,
}

#[derive(Clap)]
struct Set {
    #[clap(subcommand)]
    subcmd: SetE,
}

#[derive(Clap)]
enum SetE {
    Color(SetColor),
}

#[derive(Clap)]
struct SetColor {
    body: String,
    buttons: String,
    left_grip: Option<String>,
    right_grip: Option<String>,
}

fn main() -> Result<()> {
    let opts = Opts::parse();

    let api = HidApi::new()?;
    if let Some(device_info) = api
        .device_list()
        .find(|x| x.vendor_id() == NINTENDO_VENDOR_ID)
    {
        let device = device_info.open_device(&api)?;
        let mut joycon = JoyCon::new(device, device_info.clone())?;

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
            SubCommand::Calibrate(calib) => match calib.subcmd {
                CalibrateE::Sticks => calibrate_sticks(&mut joycon)?,
                CalibrateE::Gyroscope => unimplemented!(),
            },
            SubCommand::Get => get(&mut joycon)?,
            SubCommand::Set(set) => match set.subcmd {
                SetE::Color(arg) => set_color(&mut joycon, arg)?,
            },
            SubCommand::Monitor => monitor(&mut joycon)?,
        }
    } else {
        eprintln!("No device found");
    }
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
    let color = joycon.read_spi(RANGE_CONTROLLER_COLOR)?;
    let color = color.color().unwrap();
    println!("  body: {}", color.body);
    println!("  buttons: {}", color.buttons);
    if dev_info.use_spi_colors == UseSPIColors::IncludingGrip {
        println!("  left grip: {}", color.left_grip);
        println!("  right grip: {}", color.right_grip);
    }
    println!();

    let imu_factory_result = joycon.read_spi(RANGE_FACTORY_CALIBRATION_SENSORS)?;
    let imu_factory_settings = imu_factory_result.imu_factory_calib().unwrap();
    let imu_user_result = joycon.read_spi(RANGE_USER_CALIBRATION_SENSORS)?;
    let imu_user_settings = imu_user_result.imu_user_calib().unwrap();

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

    let sticks_factory_result = joycon.read_spi(RANGE_FACTORY_CALIBRATION_STICKS)?;
    let sticks_factory_settings = sticks_factory_result.sticks_factory_calib().unwrap();
    let sticks_user_result = joycon.read_spi(RANGE_USER_CALIBRATION_STICKS)?;
    let sticks_user_settings = sticks_user_result.sticks_user_calib().unwrap();
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

fn set_color(joycon: &mut JoyCon, arg: SetColor) -> Result<()> {
    let dev_info = joycon.get_dev_info()?;
    let is_pro_controller = dev_info.which_controller == WhichController::ProController;

    let mut colors = ControllerColor {
        body: arg.body.parse()?,
        buttons: arg.buttons.parse()?,
        ..Default::default()
    };
    if let (Some(left), Some(right)) = (arg.left_grip, arg.right_grip) {
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
        for frame in &report.imu.unwrap() {
            orientation = orientation
                * Quaternion::from(Euler::new(
                    Deg(frame.gyro.y * 0.005),
                    Deg(frame.gyro.z * 0.005),
                    Deg(frame.gyro.x * 0.005),
                ));
        }
        if now.elapsed() > Duration::from_millis(500) {
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
        }
    }
}
