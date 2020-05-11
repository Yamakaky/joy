use joycon_sys::mcu::ir_register::*;
use joycon_sys::mcu::*;
use joycon_sys::*;

pub struct Image {
    buffer: Box<[[u8; 300]; 0x100]>,
    resolution: Resolution,
}

impl Image {
    pub fn new(resolution: Resolution) -> Image {
        Image {
            buffer: Box::new([[0; 300]; 0x100]),
            resolution: resolution,
        }
    }

    pub fn handle(&mut self, packet: &IRData) -> OutputReport {
        // TODO: handle lossed packets
        self.buffer[packet.frag_number as usize] = packet.img_fragment;
        println!("got packet {}", packet.frag_number);
        if packet.frag_number == self.resolution.max_fragment_id() {
            println!("got complete packet");
        }
        OutputReport::ir_ack(packet.frag_number)
    }
}
