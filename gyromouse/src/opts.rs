use std::str::FromStr;

use clap::Clap;

#[derive(Debug, Clap)]
pub struct Opts {
    #[clap(short, long)]
    pub backend: Option<Backend>,
    #[clap(subcommand)]
    pub cmd: Cmd,
}

#[derive(Debug, Clap)]
pub enum Backend {
    #[cfg(feature = "sdl2")]
    Sdl,
    #[cfg(feature = "hidapi")]
    Hid,
}

#[derive(Debug, Clap)]
pub enum Cmd {
    Validate(Run),
    FlickCalibrate,
    Run(Run),
    List,
}

#[derive(Debug, Clap)]
pub struct Run {
    pub mapping_file: String,
}

impl FromStr for Backend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            #[cfg(feature = "sdl2")]
            "sdl" => Ok(Backend::Sdl),
            #[cfg(feature = "hidapi")]
            "hid" => Ok(Backend::Hid),
            _ => Err(format!("unknown backend: {}", s)),
        }
    }
}
