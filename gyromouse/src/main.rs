mod backend;
mod calibration;
mod config;
mod engine;
mod gyromouse;
mod joystick;
mod mapping;
mod mouse;
mod opts;
mod space_mapper;

use std::time::Instant;

use clap::Clap;
use enum_map::EnumMap;
use hid_gamepad::sys::{JoyKey, KeyStatus};
use mapping::Buttons;
use opts::Opts;

#[derive(Debug, Copy, Clone)]
pub enum ClickType {
    Press,
    Release,
    Click,
    Toggle,
}

impl ClickType {
    pub fn apply(self, val: bool) -> bool {
        match self {
            ClickType::Press => false,
            ClickType::Release => true,
            ClickType::Click => unimplemented!(),
            ClickType::Toggle => !val,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    #[cfg(feature = "sdl")]
    return backend::sdl::sdl_main(&opts);
    #[cfg(feature = "hidapi")]
    return backend::hidapi::hidapi_main(&opts);
}
