#[cfg(feature = "sdl")]
pub mod sdl;

#[cfg(feature = "hidapi")]
pub mod hidapi;

pub trait Backend {
    
}