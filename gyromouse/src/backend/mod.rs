use crate::{config::settings::Settings, mapping::Buttons, opts::Run};

#[cfg(feature = "sdl2")]
pub mod sdl;

#[cfg(feature = "hidapi")]
pub mod hidapi;

pub trait Backend {
    fn list_devices(&mut self) -> anyhow::Result<()>;
    fn run(&mut self, opts: Run, settings: Settings, bindings: Buttons) -> anyhow::Result<()>;
}
