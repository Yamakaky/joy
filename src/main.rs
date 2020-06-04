use hidapi::HidApi;
use iced_winit::winit;
use iced_winit::winit::event_loop::*;
use joycon_sys::light;
use joycon_sys::mcu::ir::*;
use joycon_sys::output::*;
use render::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

mod calibration;
mod hid;
mod image;
mod imu_handler;
mod mouse;
mod render;

fn main() {
    env_logger::init();
    std::panic::set_hook(Box::new(|x| {
        println!("{}", x);
        std::thread::sleep(std::time::Duration::from_secs(5));
    }));

    let event_loop = EventLoop::with_user_event();
    let window = winit::window::WindowBuilder::new()
        .with_maximized(true)
        .with_title("Joy")
        .build(&event_loop)
        .unwrap();
    let proxy = event_loop.create_proxy();
    let (thread_contact, recv) = mpsc::channel();
    let thread_handle = std::thread::spawn(|| real_main(proxy, recv));

    render::run(event_loop, window, thread_contact, thread_handle);
}

fn real_main(
    proxy: EventLoopProxy<JoyconData>,
    recv: mpsc::Receiver<JoyconCmd>,
) -> anyhow::Result<()> {
    let mut api = HidApi::new()?;
    loop {
        api.refresh_devices()?;
        if let Some(device_info) = api
            .device_list()
            .filter(|x| x.vendor_id() == joycon_sys::NINTENDO_VENDOR_ID)
            .next()
        {
            let device = device_info.open_device(&api)?;
            match hid_main(device, device_info, proxy.clone(), &recv) {
                Ok(true) => {}
                Ok(false) => return Ok(()),
                Err(e) => println!("Joycon error: {}", e),
            }
        } else {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}

#[allow(dead_code, unused_mut, unused_variables)]
fn hid_main(
    device: hidapi::HidDevice,
    device_info: &hidapi::DeviceInfo,
    proxy: EventLoopProxy<JoyconData>,
    recv: &mpsc::Receiver<JoyconCmd>,
) -> anyhow::Result<bool> {
    let val = OutputReport::default();

    let mut enigo = enigo::Enigo::new();
    let mut mouse = mouse::Mouse::default();

    let resolution = Resolution::R160x120;

    let mut device = hid::JoyCon::new(device, device_info.clone(), resolution)?;
    println!("new dev: {:?}", device.get_dev_info()?);
    let mut gui_still_running = true;

    // We get the orientation of the camera just after the last frame since it
    // should be close from the capture time of the IR picture.
    let last_position = Rc::new(RefCell::new(None));
    let last_position2 = Rc::clone(&last_position);
    device.set_ir_callback(Box::new(move |image| {
        let mut last_position = last_position.borrow_mut();
        if let Err(_) = proxy.send_event(JoyconData::IRImage(
            image,
            last_position.take().unwrap_or_default(),
        )) {
            dbg!("shutdown ");
            gui_still_running = false;
        }
    }));

    println!("Calibrating...");
    device.enable_imu()?;
    device.load_calibration()?;
    println!("Running...");

    device.set_imu_callback(Box::new(move |position| {
        let mut last_position = last_position2.borrow_mut();
        if last_position.is_none() {
            *last_position = Some(*position);
        }
    }));

    dbg!(device.set_home_light(light::HomeLight::new(
        0x8,
        0x2,
        0x0,
        &[(0xf, 0xf, 0), (0x2, 0xf, 0)],
    ))?);

    device.set_player_light(light::PlayerLights::new(
        true, false, false, true, false, true, true, false,
    ))?;

    dbg!(device.set_report_mode_mcu()?);
    dbg!(device.enable_mcu()?);
    dbg!(device.set_mcu_mode_ir()?);
    device.change_ir_resolution(resolution)?;

    //device.set_imu_sens()?;
    //device.enable_imu()?;

    let mut i = 0;
    device.enable_ir_loop = true;
    while gui_still_running {
        /*

        device.max_raw_gyro = 0;
        let mouse_factor = 1920. * 8.;
        let mut sleep = false;
        for delta in &device.get_gyro_rot_delta(true)? {
            if sleep {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            sleep = true;
            rotation += *delta;
            mouse.move_relative(&mut enigo, -delta.2 * mouse_factor, delta.1 * mouse_factor);
        }*/
        let stick = device.get_sticks()?;

        if i % 60 == 0 {
            println!("joycon thread still running");
        }
        i += 1;

        //println!("{:?}", stick);

        while let Ok(cmd) = recv.try_recv() {
            match cmd {
                JoyconCmd::Stop => {
                    eprintln!("shutting down thread");
                    gui_still_running = false;
                }
                JoyconCmd::SetResolution(resolution) => {
                    dbg!(device.change_ir_resolution(resolution)?);
                }
                JoyconCmd::SetRegister(register) => {
                    assert!(!register.same_address(Register::resolution(Resolution::R320x240)));
                    dbg!(device.set_ir_registers(&[register, Register::finish(),])?);
                }
                JoyconCmd::SetRegisters([r1, r2]) => {
                    assert!(!r1.same_address(Register::resolution(Resolution::R320x240)));
                    assert!(!r2.same_address(Register::resolution(Resolution::R320x240)));
                    dbg!(device.set_ir_registers(&[r1, r2, Register::finish(),])?);
                }
            }
        }
    }

    dbg!(device.set_report_mode_standard()?);
    dbg!(device.disable_mcu()?);

    device.set_player_light(light::PlayerLights::new(
        true, false, false, true, false, false, false, false,
    ))?;

    Ok(false)
}
