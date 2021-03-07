use anyhow::Context;
use bluetooth_sys::*;
use joycon::{
    hidapi::HidDevice,
    joycon_sys::{
        output::SubcommandRequestEnum, InputReport, InputReportId::StandardFull, OutputReport,
    },
};
use libc::sockaddr;
use socket2::{SockAddr, Socket};
use std::{
    convert::TryInto,
    ffi::CString,
    fs::OpenOptions,
    io::Write,
    mem::{size_of_val, zeroed},
    thread::sleep,
    time::{Duration, Instant},
};

use crate::opts::Relay;

pub fn relay(device: HidDevice, opts: &Relay) -> anyhow::Result<()> {
    let mut output = opts
        .output
        .as_ref()
        .map(|path| {
            OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
                .context("opening the log file")
        })
        .transpose()?;
    let (mut _client_ctl, mut client_itr) = connect_switch(&opts.address)?;

    // Force input reports to be generated so that we don't have to manually click on a button.
    device.write(
        OutputReport::from(SubcommandRequestEnum::SetInputReportMode(
            StandardFull.into(),
        ))
        .as_bytes(),
    )?;

    let start = Instant::now();
    loop {
        {
            let mut buf = [0; 500];
            buf[0] = 0xa1;
            let len = device
                .read_timeout(&mut buf[1..], 0)
                .context("joycon recv")?;
            if len > 0 {
                let mut report = InputReport::new();
                let raw_report = report.as_bytes_mut();
                raw_report.copy_from_slice(&buf[1..raw_report.len() + 1]);

                let elapsed = start.elapsed().as_secs_f64();

                if let Some(subcmd) = report.subcmd_reply() {
                    println!("{:0>9.4} {:?}", elapsed, subcmd);
                } else if let Some(mcu) = report.mcu_report() {
                    println!("{:0>9.4} {:?}", elapsed, mcu);
                }

                if let Some(ref mut out) = output {
                    writeln!(out, "> {:0>9.4} {}", elapsed, hex::encode(&buf[1..len + 1]))?;
                }

                if let Err(e) = client_itr.send(&buf[..len + 1]) {
                    if e.raw_os_error() == Some(107) {
                        eprintln!("Reconnecting the switch");
                        let x = connect_switch(&opts.address)?;
                        _client_ctl = x.0;
                        client_itr = x.1;

                        // Force input reports to be generated so that we don't have to manually click on a button.
                        device.write(
                            OutputReport::from(SubcommandRequestEnum::SetInputReportMode(
                                StandardFull.into(),
                            ))
                            .as_bytes(),
                        )?;
                    }
                }
            }
        }
        {
            let mut buf = [0; 500];
            if let Ok(len) = client_itr.recv(&mut buf).context("switch recv") {
                if len > 0 {
                    let mut report = OutputReport::new();
                    let raw_report = report.as_bytes_mut();
                    raw_report.copy_from_slice(&buf[1..raw_report.len() + 1]);

                    let elapsed = start.elapsed().as_secs_f64();

                    if let Some(subcmd) = report.rumble_subcmd() {
                        println!("{:0>9.4} {:?}", elapsed, subcmd);
                    } else if let Some(mcu) = report.request_mcu_data() {
                        println!("{:0>9.4} {:?}", elapsed, mcu);
                    }

                    if let Some(ref mut out) = output {
                        writeln!(out, "< {:0>9.4} {}", elapsed, hex::encode(&buf[1..len + 1]))?;
                    }

                    device.write(&buf[1..len]).context("joycon send")?;
                }
            }
        }
        sleep(Duration::from_millis(1))
    }
}

fn connect_switch(address: &str) -> anyhow::Result<(Socket, Socket)> {
    let client_ctl = Socket::new(
        (AF_BLUETOOTH as i32).into(),
        (__socket_type_SOCK_SEQPACKET as i32).into(),
        Some((BTPROTO_L2CAP as i32).into()),
    )?;
    let client_itr = Socket::new(
        (AF_BLUETOOTH as i32).into(),
        (__socket_type_SOCK_SEQPACKET as i32).into(),
        Some((BTPROTO_L2CAP as i32).into()),
    )?;

    unsafe {
        let mut addr = sockaddr_l2 {
            l2_family: AF_BLUETOOTH.try_into().unwrap(),
            // todo: watch out endian
            l2_psm: 17u16.to_le(),
            ..zeroed()
        };
        let sa = CString::new(address)?;
        str2ba(sa.as_ptr(), &mut addr.l2_bdaddr);
        let ctl_addr = SockAddr::from_raw_parts(
            &addr as *const _ as *const sockaddr,
            size_of_val(&addr) as u32,
        );
        client_ctl
            .connect(&ctl_addr)
            .context("error connecting psm 17")?;
        client_ctl
            .set_nonblocking(true)
            .context("non blocking error")?;

        addr.l2_psm = 19u16.to_le();
        let itr_addr = SockAddr::from_raw_parts(
            &addr as *const _ as *const sockaddr,
            size_of_val(&addr) as u32,
        );
        client_itr
            .connect(&itr_addr)
            .context("error connecting psm 17")?;
        client_itr
            .set_nonblocking(true)
            .context("non blocking error")?;
    }

    Ok((client_ctl, client_itr))
}
