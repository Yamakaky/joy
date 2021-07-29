use std::{collections::HashMap, fs::File, io::Read, time::Instant};

use anyhow::Context;
use sdl2::{
    self, controller::GameController, event::Event, keyboard::Keycode, GameControllerSubsystem,
};

use crate::{
    calibration::Calibration,
    config::{self, settings::Settings},
    engine::Engine,
    mapping::Buttons,
    opts::Opts,
};

pub fn sdl_main(opts: &Opts) -> anyhow::Result<()> {
    let opts = match opts {
        Opts::Run(r) => r,
        _ => todo!(),
    };
    let sdl = sdl2::init().unwrap();
    let game_controller_system = sdl.game_controller().unwrap();

    let mut event_pump = sdl.event_pump().unwrap();

    let mut controllers = HashMap::new();

    'running: loop {
        let now = Instant::now();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::ControllerDeviceAdded { which, .. } => {
                    let controller = game_controller_system.open(which)?;
                    dbg!(controller.name());
                    let mut settings = Settings::default();
                    let mut bindings = Buttons::new();
                    let mut content_file = File::open(&opts.mapping_file)
                        .with_context(|| format!("opening config file {}", opts.mapping_file))?;
                    let content = {
                        let mut buf = String::new();
                        content_file.read_to_string(&mut buf)?;
                        buf
                    };
                    config::parse::parse_file(&content, &mut settings, &mut bindings).unwrap();

                    let mut calibration = Calibration::with_capacity(250 * 60);
                    let mut engine = Engine::new(settings, bindings, calibration);
                    controllers.insert(which, ControllerState { controller, engine });
                }
                Event::ControllerDeviceRemoved { which, .. } => {
                    controllers.remove(&which);
                }
                Event::ControllerButtonDown {
                    timestamp: _,
                    which,
                    button,
                } => {
                    let controller = &controllers[&which];
                    //controller.engine.tick(report)
                }
                Event::ControllerButtonUp {
                    timestamp: _,
                    which,
                    button,
                } => {}
                _ => {}
            }
        }
    }

    Ok(())
}

struct ControllerState {
    controller: GameController,
    engine: Engine,
}
