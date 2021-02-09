use bluetooth_sys::*;
use std::{convert::TryInto, mem::zeroed};

fn main() {
    unsafe {
        let client_ctl = socket(
            AF_BLUETOOTH.try_into().unwrap(),
            __socket_type_SOCK_SEQPACKET.try_into().unwrap(),
            BTPROTO_L2CAP.try_into().unwrap(),
        );
        let client_itr = socket(
            AF_BLUETOOTH.try_into().unwrap(),
            __socket_type_SOCK_SEQPACKET.try_into().unwrap(),
            BTPROTO_L2CAP.try_into().unwrap(),
        );
        let mut addr = sockaddr_l2 {
            l2_family: AF_BLUETOOTH.try_into().unwrap(),
            // todo: watch out endian
            l2_psm: 17,
            ..zeroed()
        };
        str2ba(
            b"58:2F:40:DF:31:31\0".as_ptr() as *const i8,
            &mut addr.l2_bdaddr,
        );
        assert_eq!(
            0,
            connect(
                client_ctl,
                &addr as *const _ as *const sockaddr,
                std::mem::size_of_val(&addr).try_into().unwrap(),
            )
        );
        addr.l2_psm = 19;
        assert_eq!(
            0,
            connect(
                client_itr,
                &addr as *const _ as *const sockaddr,
                std::mem::size_of_val(&addr).try_into().unwrap(),
            )
        );
    }
}
