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
        self.send_subcmd_wait(proto::OutputReport {
            packet_counter: 0,
            report_id: proto::OutputReportId::RumbleSubcmd.into(),
            rumble_data: proto::RumbleData::default(),
            subcmd: proto::SubcommandRequest {
                subcommand_id: proto::SubcommandId::EnableIMU.into(),
                u: proto::SubcommandRequestData { nothing: () },
            },
        })
    }

    pub fn set_standard_mode(&mut self) {
        self.send_subcmd_wait(proto::OutputReport {
            packet_counter: 0,
            report_id: proto::OutputReportId::RumbleSubcmd,
            rumble_data: proto::RumbleData::default(),
            subcmd: proto::SubcommandRequest {
                subcommand_id: proto::SubcommandId::SetInputReportMode,
                u: proto::SubcommandRequestData {
                    input_report_mode: proto::InputReportMode::StandardFull,
                }
            }
        })
    }

    pub fn set_player_light(&mut self, player_lights: proto::PlayerLights) {
        self.send_subcmd_wait(proto::OutputReport {
            packet_counter: 0,
            report_id: proto::OutputReportId::RumbleSubcmd,
            rumble_data: proto::RumbleData::default(),
            subcmd: proto::SubcommandRequest {
                subcommand_id: proto::SubcommandId::SetPlayerLights,
                u: proto::SubcommandRequestData {
                    player_lights
                } 
            }
        })
    }

    fn send_subcmd_wait(&mut self, mut out_report: proto::OutputReport) {
        self.send(&mut out_report);
        let mut buffer = [0u8; 5999];
        // TODO: loop limit
        loop {
            let in_report = self.recv(&mut buffer);
            unsafe {
                if in_report.report_id.try_into().unwrap() == proto::InputReportId::Standard &&
                in_report.u.standard.u.subcmd_reply.subcommand_id.try_into().unwrap() == out_report.subcmd.subcommand_id {
                    break;
                }
            }
        }
    }
}
