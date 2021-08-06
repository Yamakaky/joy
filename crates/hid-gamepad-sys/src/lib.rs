pub use hid_gamepad_types::*;

pub trait GamepadDriver {
    fn init(
        &self,
        api: &hidapi::HidApi,
        device_info: &hidapi::DeviceInfo,
    ) -> anyhow::Result<Option<Box<dyn GamepadDevice>>>;
}

pub trait GamepadDevice {
    fn recv(&mut self) -> anyhow::Result<Report>;
    fn as_any(&mut self) -> &mut dyn std::any::Any;
}
