use crate::proto;

pub struct JoyCon {
    device: hidapi::HidDevice,
    counter: u8,
}

impl JoyCon {
    pub fn new(hid_device: hidapi::HidDevice) -> JoyCon {
        JoyCon {
            device: hid_device,
            counter: 42,
        }
    }
    pub fn send(&mut self, report: &mut proto::OutputReport) {
        report.packet_counter = self.counter;
        self.counter += 1;
        let raw_data = unsafe {
            std::slice::from_raw_parts(
                (report as *const proto::OutputReport).cast::<u8>(),
                std::mem::size_of_val(report),
            )
        };
        self.device.write(raw_data).expect("write");
    }

    pub fn recv(&self, buffer: &mut [u8]) -> &proto::InputReport {
        let size = self.device.read(buffer).expect("read");
        let buf = &buffer[..size];
        let response = unsafe { std::mem::transmute(&buf[0]) };
        response
    }

    pub fn enable_imu(&mut self) {
        // enable IMU
        let mut out_report = proto::OutputReport {
            packet_counter: 0,
            report_id: proto::OutputReportId::RumbleSubcmd.into(),
            rumble_data: proto::RumbleData::default(),
            subcmd: proto::SubcommandRequest {
                subcommand_id: proto::SubcommandId::EnableIMU.into(),
                u: proto::SubcommandRequestData { nothing: () },
            },
        };
        self.send(&mut out_report);
        self.wait_subcmd_id(out_report.subcmd.subcommand_id);
    }

    pub fn set_standard_mode(&mut self) {
        let mut out_report = proto::OutputReport {
            packet_counter: 0,
            report_id: proto::OutputReportId::RumbleSubcmd.into(),
            rumble_data: proto::RumbleData::default(),
            subcmd: proto::SubcommandRequest {
                subcommand_id: proto::SubcommandId::SetInputReportMode.into(),
                u: proto::SubcommandRequestData {
                    input_report_mode: proto::InputReportMode::StandardFull,
                }
            }
        };
        self.send(&mut out_report);
        self.wait_subcmd_id(out_report.subcmd.subcommand_id);
    }

    fn wait_subcmd_id(&self, id: proto::SubcommandId) {
        let mut buffer = [0u8; 5999];
        // TODO: loop limit
        loop {
            let report = self.recv(&mut buffer);
            unsafe {
                if report.report_id.try_into().unwrap() == proto::InputReportId::Standard &&
                report.u.standard.u.subcmd_reply.subcommand_id.try_into().unwrap() == id {
                    break;
                }
            }
        }
    }
}
