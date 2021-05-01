use anyhow::Result;
use image::GrayImage;
use joycon::{joycon_sys::mcu::ir::Resolution, JoyCon};
use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalSize},
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
};
use winit_input_helper::WinitInputHelper;

#[derive(Debug)]
enum Cmd {
    Image(GrayImage),
    Stop,
}

pub fn run(mut joycon: JoyCon) -> Result<()> {
    joycon.enable_imu()?;
    joycon.enable_ir(Resolution::R160x120)?;

    let event_loop = EventLoop::with_user_event();

    std::thread::spawn({
        let proxy = event_loop.create_proxy();
        move || -> Result<()> {
            loop {
                let report = joycon.tick()?;
                if let Some(img) = report.image {
                    proxy.send_event(Cmd::Image(img))?;
                }
            }
        }
    });

    let mut input = WinitInputHelper::new();
    let (window, p_width, p_height, mut _hidpi_factor) =
        create_window("Joycon camera", &event_loop);

    let surface_texture = SurfaceTexture::new(p_width, p_height, &window);

    let mut image = None;

    let (p_width, p_height) = (300, 400);
    let mut pixels = Pixels::new(p_width, p_height, surface_texture)?;
    event_loop.run(move |event, _, control_flow| {
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }
            // Adjust high DPI factor
            if let Some(factor) = input.scale_factor_changed() {
                _hidpi_factor = factor;
            }
            // Resize the window
            if let Some(size) = input.window_resized() {
                pixels.resize(size.width, size.height);
            }
        } else if let Event::UserEvent(cmd) = event {
            match cmd {
                Cmd::Image(img) => {
                    image = Some(img);
                    window.request_redraw();
                }
                Cmd::Stop => {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }
        } else if let Event::RedrawRequested(_) = event {
            let frame = pixels.get_frame();
            frame.fill(0);

            if let Some(ref img) = image {
                for (x, y, pixel) in img.enumerate_pixels() {
                    let offset = ((x + y * p_width) * 4) as usize;
                    let color = &mut frame[offset..offset + 4];
                    color[0] = 0;
                    color[1] = 0;
                    color[2] = pixel.0[0];
                    color[3] = 255;
                }
            }

            if pixels
                .render()
                .map_err(|e| eprintln!("pixels.render() failed: {}", e))
                .is_err()
            {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }
    });
}

const SCREEN_WIDTH: u32 = 600;
const SCREEN_HEIGHT: u32 = 600;

fn create_window(
    title: &str,
    event_loop: &EventLoop<Cmd>,
) -> (winit::window::Window, u32, u32, f64) {
    // Create a hidden window so we can estimate a good default window size
    let window = winit::window::WindowBuilder::new()
        .with_visible(false)
        .with_title(title)
        .build(&event_loop)
        .unwrap();
    let hidpi_factor = window.scale_factor();

    // Get dimensions
    let width = SCREEN_WIDTH as f64;
    let height = SCREEN_HEIGHT as f64;
    let (monitor_width, monitor_height) = {
        if let Some(monitor) = window.current_monitor() {
            let size = monitor.size().to_logical(hidpi_factor);
            (size.width, size.height)
        } else {
            (width, height)
        }
    };
    let scale = (monitor_height / height * 2.0 / 3.0).round().max(1.0);

    // Resize, center, and display the window
    let min_size: winit::dpi::LogicalSize<f64> =
        PhysicalSize::new(width, height).to_logical(hidpi_factor);
    let default_size = LogicalSize::new(width * scale, height * scale);
    let center = LogicalPosition::new(
        (monitor_width - width * scale) / 2.0,
        (monitor_height - height * scale) / 2.0,
    );
    window.set_inner_size(default_size);
    window.set_min_inner_size(Some(min_size));
    window.set_outer_position(center);
    window.set_visible(true);

    let size = default_size.to_physical::<f64>(hidpi_factor);

    (
        window,
        size.width.round() as u32,
        size.height.round() as u32,
        hidpi_factor,
    )
}
