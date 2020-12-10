use clap::Clap;
use joycon::{
    hidapi::HidApi,
    joycon_sys::{
        input::{BatteryLevel, Stick},
        light::{self, PlayerLight},
        NINTENDO_VENDOR_ID,
    },
    JoyCon,
};
use std::thread::sleep;
use std::time::Duration;

#[derive(Clap)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    #[clap(version = "1.0")]
    Calibrate(Calibrate),
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

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();

    let api = HidApi::new()?;
    if let Some(device_info) = api
        .device_list()
        .find(|x| x.vendor_id() == NINTENDO_VENDOR_ID)
    {
        let device = device_info.open_device(&api)?;
        let mut joycon = JoyCon::new(device, device_info.clone())?;

        dbg!(joycon.set_home_light(light::HomeLight::new(
            0x8,
            0x2,
            0x0,
            &[(0xf, 0xf, 0), (0x2, 0xf, 0)],
        ))?);

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
        }
    } else {
        eprintln!("No device found");
    }
    Ok(())
}

fn calibrate_sticks(joycon: &mut JoyCon) -> anyhow::Result<()> {
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

fn raw_sticks(joycon: &mut JoyCon) -> anyhow::Result<(Stick, Stick)> {
    let report = joycon.recv()?;
    let std_report = report.standard().expect("should be standard");
    Ok((std_report.left_stick, std_report.right_stick))
}
