use hidapi::HidApi;
use joycon_sys::light;
use joycon_sys::mcu::ir::*;
use joycon_sys::output::*;
use render::JoyconCmd;
use std::rc::Rc;
use std::sync::mpsc;
use winit::event_loop::*;

mod calibration;
mod hid;
mod image;
mod imu_handler;
mod mouse;
mod render;

fn main() -> ! {
    let event_loop = EventLoop::with_user_event();
    let window = winit::window::Window::new(&event_loop).unwrap();
    let proxy = event_loop.create_proxy();
    let (thread_contact, recv) = mpsc::channel();
    let thread_handle = std::thread::spawn(|| hid_main(proxy, recv));

    futures::executor::block_on(render::run(
        event_loop,
        window,
        thread_contact,
        thread_handle,
    ))
}

#[allow(dead_code, unused_mut, unused_variables)]
fn hid_main(
    proxy: EventLoopProxy<render::IRData>,
    recv: mpsc::Receiver<JoyconCmd>,
) -> anyhow::Result<()> {
    let val = OutputReport::default();

    let api = HidApi::new()?;
    let mut enigo = enigo::Enigo::new();
    let mut mouse = mouse::Mouse::default();
    let proxy_rc = Rc::new(proxy);

    for device in api
        .device_list()
        .filter(|x| x.vendor_id() == joycon_sys::NINTENDO_VENDOR_ID)
    {
        let resolution = Resolution::R160x120;

        let mut device = hid::JoyCon::new(device.open_device(&api)?, device.clone(), resolution)?;
        println!("new dev: {:?}", device.get_dev_info()?);
        let mut gui_still_running = true;

        let proxy = proxy_rc.clone();
        device.set_ir_callback(Box::new(move |buffer, width, height| {
            if let Err(_) = proxy.send_event(render::IRData {
                buffer,
                width,
                height,
            }) {
                dbg!("shutdown ");
                gui_still_running = false;
            }
        }));

        println!("Calibrating...");
        device.enable_imu()?;
        device.load_calibration()?;
        println!("Running...");

        let mut i2 = 0;
        device.set_imu_callback(Box::new(move |position| {
            if i2 % 200 == 0 {
                // dbg!(position.rotation * cgmath::Vector3::new(1., 0., 0.));
            }
            i2 += 1;
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
                }
            }
        }

        dbg!(device.set_report_mode_standard()?);
        dbg!(device.disable_mcu()?);

        device.set_player_light(light::PlayerLights::new(
            true, false, false, true, false, false, false, false,
        ))?;

        break;
    }
    Ok(())
}
