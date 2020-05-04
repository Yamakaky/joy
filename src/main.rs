#[macro_use]
extern crate num_derive;

use hidapi::HidApi;

mod hid;
mod proto;

const NINTENDO_VENDOR_ID: u16 = 1406;
const _JOYCON_L_BT: u16 = 0x2007;

fn main() {
    let mut buffer = [0u8; 5999];
    println!("stop");
    match HidApi::new() {
        Ok(api) => {
            for device in api.device_list().filter(|x| x.vendor_id() == NINTENDO_VENDOR_ID) {
                let mut device = hid::JoyCon::new(device.open_device(&api).expect("open"));
                println!("new dev");
                device.enable_imu();
                device.set_standard_mode();
                device.set_player_light(proto::PlayerLights::new(true, false, false, true, false, false, false, false));
                let mut i = 0;
                loop {
                    let report = device.recv(&mut buffer);
                    println!("{:?}", report);
                    i += 1;
                    if i > 4 {
                        break;
                    }
                }
            }
        },
        Err(e) => {
            eprintln!("Error: {}", e);
        },
    }
    println!("done");
}
