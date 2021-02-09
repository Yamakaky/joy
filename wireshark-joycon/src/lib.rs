#![allow(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    improper_ctypes
)]

use std::{
    convert::TryInto,
    ffi::{c_void, CString},
    os::raw::{c_int, c_uint},
};

#[no_mangle]
pub static plugin_version: &[u8; 6] = b"0.0.0\0";
#[no_mangle]
pub static plugin_want_major: c_uint = WIRESHARK_VERSION_MAJOR;
#[no_mangle]
pub static plugin_want_minor: c_uint = WIRESHARK_VERSION_MINOR;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

static mut proto: c_int = -1;

unsafe extern "C" fn dissect_foo(
    tvb: *mut tvbuff,
    packet_info: *mut _packet_info,
    _proto_tree: *mut _proto_node,
    _data: *mut c_void,
) -> i32 {
    dbg!("prout");
    let name = CString::new("FOO").unwrap();
    col_set_str(
        (*packet_info).cinfo,
        COL_PROTOCOL.try_into().unwrap(),
        name.as_ptr(),
    );
    col_clear((*packet_info).cinfo, COL_INFO.try_into().unwrap());
    tvb_captured_length(tvb).try_into().unwrap()
}

unsafe extern "C" fn proto_register_foo() {
    dbg!("pat");
    let name = CString::new("FOO Protocol").unwrap();
    let short_name = CString::new("FOO").unwrap();
    let filter_name = CString::new("foo").unwrap();
    proto = proto_register_protocol(name.as_ptr(), short_name.as_ptr(), filter_name.as_ptr());
}

unsafe extern "C" fn proto_reg_handoff_foo() {
    dbg!("fdsfds");
    let handle = create_dissector_handle(Some(dissect_foo), proto);
    let psm = CString::new("btl2cap.psm").unwrap();
    dissector_add_uint(psm.as_ptr(), 17, handle);
    dissector_add_uint(psm.as_ptr(), 19, handle);
}

#[no_mangle]
pub unsafe extern "C" fn plugin_register() {
    let plugin_foo = proto_plugin {
        register_protoinfo: Some(proto_register_foo),
        register_handoff: Some(proto_reg_handoff_foo),
    };
    proto_register_plugin(&plugin_foo);
    dbg!("sfds");
}
