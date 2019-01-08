#[macro_use]
extern crate num_derive;

use hidapi::HidApi;

mod proto;

const NINTENDO_VENDOR_ID: u16 = 1406;
const JOYCON_L_BT: u16 = 0x2007;

fn main() {
    match HidApi::new() {
        Ok(api) => {
            for device in api.devices().iter().filter(|x| x.vendor_id == NINTENDO_VENDOR_ID) {
                let device = device.open_device(&api).expect("open");
                let mut i = 0;
                loop {
                    let mut buf = [0u8; 5999];
                    let size = device.read(&mut buf).expect("read");
                    let buf = &buf[..size];
                    let data: &proto::InputReport = unsafe { std::mem::transmute(&buf[0]) };
                    println!("{:?}", data);
                    i+= 1;
                    if i > 40 {
                        break;
                    }
                }
            }
        },
        Err(e) => {
            eprintln!("Error: {}", e);
        },
    }
}
