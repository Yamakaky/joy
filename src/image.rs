use joycon_sys::mcu::*;
use joycon_sys::*;

pub struct Image {
    buffer: Box<[[u8; 300]; 0x100]>,
    resolution: ir::Resolution,
    prev_fragment_id: u8,
}

impl Image {
    pub fn new(resolution: ir::Resolution) -> Image {
        Image {
            buffer: Box::new([[0; 300]; 0x100]),
            resolution: resolution,
            prev_fragment_id: 0,
        }
    }

    pub fn handle(&mut self, report: &MCUReport) -> [Option<OutputReport>; 2] {
        // TODO: handle lossed packets
        if let Some(packet) = report.as_ir_data() {
            self.buffer[packet.frag_number as usize] = packet.img_fragment;
            let resend = if packet.frag_number > 0
                && self.prev_fragment_id > 0
                && packet.frag_number - 1 > self.prev_fragment_id
            {
                println!("requesting again packet {}", packet.frag_number - 1);
                Some(OutputReport::ir_resend(packet.frag_number - 1))
            } else {
                None
            };
            //println!("got packet {}", packet.frag_number);
            if packet.frag_number == self.resolution.max_fragment_id() {
                if self.prev_fragment_id != 0 {
                    println!("got complete packet");
                    let (width, height) = self.resolution.size();
                    let mut image = image::ImageBuffer::new(width, height);
                    for (i, frag) in self
                        .buffer
                        .iter()
                        .enumerate()
                        .take(self.resolution.max_fragment_id() as usize + 1)
                    {
                        for (j, pixel) in frag.iter().cloned().enumerate() {
                            let sum = (i * 300 + j) as u32;
                            image.put_pixel(sum % width, sum / width, image::Luma([pixel]));
                        }
                    }
                    image.save("D:\\ir.png").unwrap();
                    self.buffer = Box::new([[0; 300]; 0x100]);
                }
                self.prev_fragment_id = 0;
            } else {
                self.prev_fragment_id = packet.frag_number;
            }
            [Some(OutputReport::ir_ack(packet.frag_number)), resend]
        } else if report.id == MCUReportId::Empty {
            [
                Some(OutputReport::ir_resend(self.prev_fragment_id + 1)),
                None,
            ]
        } else if report.id == MCUReportId::EmptyAwaitingCmd {
            [Some(OutputReport::ir_ack(self.prev_fragment_id)), None]
        } else {
            [None, None]
        }
    }
}
