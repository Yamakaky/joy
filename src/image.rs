use joycon_sys::mcu::*;
use joycon_sys::*;

pub struct Image {
    buffer: Box<[[u8; 300]; 0x100]>,
    resolution: ir::Resolution,
    prev_fragment_id: u8,
    cb: Option<Box<dyn FnMut(Box<[u8]>, u32, u32)>>,
}

impl Image {
    pub fn new(resolution: ir::Resolution) -> Image {
        Image {
            buffer: Box::new([[0; 300]; 0x100]),
            resolution: resolution,
            prev_fragment_id: 0,
            cb: None,
        }
    }

    pub fn set_cb(&mut self, cb: Box<dyn FnMut(Box<[u8]>, u32, u32)>) {
        self.cb = Some(cb);
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
                    if let Some(ref mut cb) = self.cb {
                        let (width, height) = self.resolution.size();
                        let mut buffer = Vec::with_capacity((width * height) as usize);
                        for fragment in self
                            .buffer
                            .iter()
                            .take(self.resolution.max_fragment_id() as usize + 1)
                        {
                            buffer.extend(fragment.iter());
                        }
                        cb(buffer.into(), width, height);
                    }
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
