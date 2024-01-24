var srcIndex = JSON.parse('{\
"dualshock":["",[],["lib.rs"]],\
"dualshock_sys":["",[],["input.rs","lib.rs","output.rs"]],\
"hid_gamepad":["",[],["error.rs","lib.rs"]],\
"hid_gamepad_sys":["",[],["lib.rs"]],\
"hid_gamepad_types":["",[],["lib.rs"]],\
"joy_music":["",[],["main.rs"]],\
"joycon":["",[],["calibration.rs","hid.rs","image.rs","imu_handler.rs","lib.rs"]],\
"joycon_sys":["",[["input",[],["mod.rs","report.rs","values.rs"]],["mcu",[],["ir.rs","ir_register.rs","mod.rs"]],["output",[],["mod.rs","report.rs","rumble.rs"]]],["accessory.rs","common.rs","imu.rs","lib.rs","light.rs","spi.rs"]],\
"joytk":["",[],["camera.rs","main.rs","opts.rs","relay.rs"]]\
}');
createSrcSidebar();
