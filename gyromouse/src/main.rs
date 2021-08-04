#![cfg_attr(test, allow(dead_code, unreachable_code, unused_variables))]

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

use std::{fs::File, io::Read};

use anyhow::Context;
use backend::Backend;
use clap::Clap;
use opts::Opts;

use crate::{config::settings::Settings, mapping::Buttons, mouse::Mouse};

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
    let mut backend: Box<dyn Backend> = match opts.backend {
        #[cfg(feature = "sdl2")]
        Some(opts::Backend::Sdl) | None => Box::new(backend::sdl::SDLBackend::new()?),
        #[cfg(feature = "hidapi")]
        Some(opts::Backend::Hid) | None => Box::new(backend::hidapi::HidapiBackend::new()?),
        Some(_) | None => {
            println!("A backend must be enabled");
            return Ok(());
        }
    };

    let mut mouse = Mouse::new();

    match opts.cmd {
        opts::Cmd::Validate(v) => {
            let mut settings = Settings::default();
            let mut bindings = Buttons::new();
            let mut content_file = File::open(&v.mapping_file)
                .with_context(|| format!("opening config file {}", v.mapping_file))?;
            let content = {
                let mut buf = String::new();
                content_file.read_to_string(&mut buf)?;
                buf
            };
            config::parse::parse_file(&content, &mut settings, &mut bindings, &mut mouse)?;
            Ok(())
        }
        opts::Cmd::FlickCalibrate => todo!(),
        opts::Cmd::Run(r) => {
            let mut settings = Settings::default();
            let mut bindings = Buttons::new();
            let mut content_file = File::open(&r.mapping_file)
                .with_context(|| format!("opening config file {}", r.mapping_file))?;
            let content = {
                let mut buf = String::new();
                content_file.read_to_string(&mut buf)?;
                buf
            };
            config::parse::parse_file(&content, &mut settings, &mut bindings, &mut mouse)?;
            backend.run(r, settings, bindings, mouse)
        }
        opts::Cmd::List => backend.list_devices(),
    }
}
