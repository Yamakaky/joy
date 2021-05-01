use std::path::PathBuf;

use clap::Clap;

/// Access every feature of the Nintendo Switch controllers
///
/// Env variables:
///
/// - `RUST_LOG=<level>`:
///
///   -   `trace`: log every bluetooth packet
///
///   -   `debug`: only log important packets
///
/// - `LOG_PRETTY=1`: use a more verbose logging format
///
/// - `LOG_TIMING=1`: show timings
#[derive(Clap)]
pub struct Opts {
    #[clap(subcommand)]
    pub subcmd: SubCommand,
    /// Wait for a controller to connect
    #[clap(short, long)]
    pub wait: bool,
}

#[derive(Clap)]
pub enum SubCommand {
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
    PulseRate,
    /// Dump the memory of the controller to a binary file
    Dump,
    /// Restore the memory of the controller from a dump file
    Restore,
    /// Decode raw reports exchanged between the controller and the Switch
    ///
    /// See the `relay` subcommand to record new traces.
    ///
    /// See the `trace/` folder for recorded dumps, and
    /// [relay_joycon.py](https://github.com/Yamakaky/joycontrol/blob/capture-text-file/scripts/relay_joycon.py)
    /// for capturing new dumps.
    Decode,
    /// Relay the bluetooth trafic between a controller and the Switch
    ///
    /// Important commands are decoded and shown, and a full log can be recorded.
    /// See the `decode` subcommand to decode logs.
    Relay(Relay),
    /// Ringcon-specific actions
    Ringcon(Ringcon),
    Camera,
}

#[derive(Clap)]
pub struct Calibrate {
    #[clap(subcommand)]
    pub subcmd: CalibrateE,
}

#[derive(Clap)]
pub enum CalibrateE {
    /// Calibrate the sticks
    Sticks,
    /// Calibrate the gyroscope
    Gyroscope,
    /// Reset gyroscope and sticks calibration to factory values
    Reset,
}

#[derive(Clap)]
pub struct Set {
    #[clap(subcommand)]
    pub subcmd: SetE,
}

#[derive(Clap)]
pub enum SetE {
    /// Change the color of the controller
    ///
    /// This is used by the switch for the controller icons. Every color is in `RRGGBB` format.
    Color(SetColor),
}

#[derive(Clap)]
pub struct SetColor {
    /// Color of the body of the controller
    pub body: String,
    /// Color of the buttons, sticks and triggers
    pub buttons: String,
    /// Color of the left grip (Pro Controller only)
    pub left_grip: Option<String>,
    /// Color of the right grip (Pro Controller only)
    pub right_grip: Option<String>,
}

#[derive(Clap)]
pub struct Ringcon {
    #[clap(subcommand)]
    pub subcmd: RingconE,
}

#[derive(Clap)]
pub enum RingconE {
    /// Get the number of flex stored in the ringcon
    StoredFlex,
    /// Show the flex value in realtime
    Monitor,
    /// Random experiments
    Exp,
}

#[derive(Clap)]
pub struct Relay {
    /// Bluetooth MAC address of the Switch
    #[clap(short, long, validator(is_mac))]
    pub address: String,
    /// Location of the log to write
    #[clap(short, long)]
    pub output: Option<PathBuf>,
    /// Decode important HID reports and print them to stdout
    #[clap(short, long)]
    pub verbose: bool,
}

fn is_mac(input: &str) -> Result<(), String> {
    let mut i = 0;
    for x in input.split(":").map(|x| u8::from_str_radix(x, 16)) {
        match x {
            Ok(_) => i += 1,
            Err(e) => return Err(format!("MAC parsing error: {}", e)),
        }
    }
    if i == 6 {
        Ok(())
    } else {
        Err("invalid MAC address".to_string())
    }
}
