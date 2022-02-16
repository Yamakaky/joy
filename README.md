# joy

Suite of tools and libraries for interactions with the Nintendo Switch controllers.

## External dependencies

On Linux, you'll need `libusb`, `libbluetooth` and `libudev`. On Ubuntu, you can install these by running:

```sh
sudo apt-get install libusb-1.0-0-dev libbluetooth-dev libudev-dev
```

## Tools

The tools can be run with `cargo run --bin <tool>`.

- `joytk`: main front-facing tool.
- `joy-infrared`: visualize the images captured by the infrared camera of the Joycon(R) as a realtime 3D view.

## Libraries

- [`joycon-sys`](https://yamakaky.github.io/joy/joycon_sys): decoding and encoding HID reports. Doesn't include any I/O.
- [`joycon`](https://yamakaky.github.io/joy/joycon): implements I/O and communication protocols on top of `joycon-sys`.
- [`dualshock`](https://yamakaky.github.io/joy/dualshock): decoding HID reports from the DS4 controller.
- [`hid-gamepad`](https://yamakaky.github.io/joy/hid_gamepad): abstraction above `dualshock` and `joycon`.
