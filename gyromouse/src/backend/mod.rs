#[cfg(feature = "sdl2")]
pub mod sdl;

#[cfg(feature = "hidapi")]
pub mod hidapi;

pub trait Backend {}
