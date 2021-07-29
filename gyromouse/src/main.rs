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

use clap::Clap;
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

    #[allow(unreachable_patterns)]
    match opts.backend {
        #[cfg(feature = "sdl")]
        Some(opts::Backend::Sdl) | None => backend::sdl::sdl_main(&opts),
        #[cfg(feature = "hidapi")]
        Some(opts::Backend::Hid) | None => backend::hidapi::hidapi_main(&opts),
        None => {
            println!("A backend must be enabled");
            Ok(())
        }
    }
}
