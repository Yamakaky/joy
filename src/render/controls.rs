use super::JoyconCmd;
use iced_core::Background;
use iced_wgpu::{widget::container, Renderer};
use iced_winit::{
    slider, Checkbox, Color, Column, Command, Container, Element, HorizontalAlignment, Length,
    Program, Radio, Row, Slider, Text, VerticalAlignment,
};
use joycon::joycon_sys::mcu::ir::*;
use std::sync::mpsc;

pub struct Controls {
    thread_contact: mpsc::Sender<JoyconCmd>,
    leds: Leds,
    max_exposure: bool,
    exposure: Sliderf32,
    far_int: Sliderf32,
    near_int: Sliderf32,
    resolution: Resolution,
    edge_smoothing: Sliderf32,
    white_threshold: Sliderf32,
    color_interpolation_threshold: Sliderf32,
    denoise: bool,
    buffer_update_time: Sliderf32,
    depth: (u32, u32, u8),
    ir_rotate: bool,
}

#[derive(Copy, Clone, Debug)]
pub enum Message {
    Leds(Leds),
    Intensity(f32, f32),
    Resolution(Resolution),
    EdgeSmoothing(f32),
    MaxExposure(bool),
    Exposure(f32),
    UpdateTime(f32),
    Depth(u32, u32, u8),
    IRRotate(bool),
    Denoise(bool),
    WhiteThreshold(f32),
    ColorInterpolationThreshold(f32),
}

impl Controls {
    pub fn new(thread_contact: mpsc::Sender<JoyconCmd>) -> Controls {
        let mut leds = Leds(0);
        leds.set_disable_far_narrow12(true);
        Controls {
            thread_contact,
            leds,
            max_exposure: false,
            exposure: Sliderf32::new(200., 600.),
            far_int: Sliderf32::new(0xf as f32, 0xf as f32),
            near_int: Sliderf32::new(0xf as f32, 0xf as f32),
            resolution: Resolution::R160x120,
            edge_smoothing: Sliderf32::new(0x23 as f32, 0xff as f32),
            white_threshold: Sliderf32::new(0xc8 as f32, 0xff as f32),
            color_interpolation_threshold: Sliderf32::new(0x44 as f32, 0xff as f32),
            denoise: true,
            buffer_update_time: Sliderf32::new(0x32 as f32, 0xff as f32),
            depth: (0, 0, 0),
            ir_rotate: true,
        }
    }

    pub fn ir_rotate(&self) -> bool {
        self.ir_rotate
    }
}

impl Program for Controls {
    type Renderer = Renderer;
    type Message = Message;

    fn update(&mut self, message: Message) -> Command<Message> {
        // TODO: debounce
        match message {
            Message::Leds(leds) => {
                self.leds = leds;
                self.thread_contact
                    .send(JoyconCmd::SetRegister(Register::ir_leds(self.leds)))
                    .unwrap();
            }
            Message::Intensity(far, near) => {
                self.far_int.value = far;
                self.near_int.value = near;
                self.thread_contact
                    .send(JoyconCmd::SetRegisters(Register::leds_intensity(
                        far as u8, near as u8,
                    )))
                    .unwrap();
            }
            Message::EdgeSmoothing(threshold) => {
                self.edge_smoothing.value = threshold;
                self.thread_contact
                    .send(JoyconCmd::SetRegister(Register::edge_smoothing_threshold(
                        threshold as u8,
                    )))
                    .unwrap();
            }
            Message::MaxExposure(max) => {
                self.max_exposure = max;
                self.thread_contact
                    .send(JoyconCmd::SetRegister(Register::exposure_mode(
                        if self.max_exposure {
                            ExposureMode::Max
                        } else {
                            ExposureMode::Manual
                        },
                    )))
                    .unwrap();
            }
            Message::Exposure(exposure) => {
                self.exposure.value = exposure;
                self.thread_contact
                    .send(JoyconCmd::SetRegisters(Register::exposure_us(
                        exposure as u32,
                    )))
                    .unwrap();
            }
            Message::UpdateTime(time) => {
                self.buffer_update_time.value = time;
                self.thread_contact
                    .send(JoyconCmd::SetRegister(Register::buffer_update_time(
                        time as u8,
                    )))
                    .unwrap();
            }
            Message::Resolution(res) => {
                self.resolution = res;
                self.thread_contact
                    .send(JoyconCmd::SetResolution(res))
                    .unwrap();
            }
            Message::Depth(x, y, depth) => self.depth = (x, y, depth),
            Message::IRRotate(rotate) => {
                self.ir_rotate = rotate;
            }
            Message::WhiteThreshold(threshold) => {
                self.white_threshold.value = threshold;
                self.thread_contact
                    .send(JoyconCmd::SetRegister(Register::white_pixel_threshold(
                        threshold as u8,
                    )))
                    .unwrap();
            }
            Message::ColorInterpolationThreshold(threshold) => {
                self.color_interpolation_threshold.value = threshold;
                self.thread_contact
                    .send(JoyconCmd::SetRegister(
                        Register::color_interpolation_threshold(threshold as u8),
                    ))
                    .unwrap();
            }
            Message::Denoise(val) => {
                self.denoise = val;
                self.thread_contact
                    .send(JoyconCmd::SetRegister(Register::denoise(val)))
                    .unwrap();
            }
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Message, Renderer> {
        let title = |s| {
            Text::new(s)
                .width(Length::Fill)
                .horizontal_alignment(HorizontalAlignment::Center)
                .size(25)
                .into()
        };

        let general_ctrl = Column::with_children(vec![
            title("General settings"),
            Checkbox::new(self.ir_rotate, "Use gyro rotation", Message::IRRotate).into(),
        ])
        .spacing(10)
        .into();

        let leds = self.leds;
        let (far_int, near_int) = (self.far_int.value, self.near_int.value);
        let leds_ctrl = Column::with_children(vec![
            title("Leds control"),
            Checkbox::new(
                !self.leds.disable_far_narrow12(),
                "Far and narrow",
                move |b| {
                    let mut leds = leds;
                    leds.set_disable_far_narrow12(!b);
                    Message::Leds(leds)
                },
            )
            .into(),
            self.far_int
                .render(
                    move |x| Message::Intensity(x, near_int),
                    format!("{}%", self.far_int.percent()),
                )
                .into(),
            Checkbox::new(
                !self.leds.disable_near_wide34(),
                "Near and wide",
                move |b| {
                    let mut leds = leds;
                    leds.set_disable_near_wide34(!b);
                    Message::Leds(leds)
                },
            )
            .into(),
            self.near_int
                .render(
                    move |x| Message::Intensity(far_int, x),
                    format!("{}%", self.near_int.percent()),
                )
                .into(),
            Checkbox::new(self.leds.strobe(), "Strobe", move |b| {
                let mut leds = leds;
                leds.set_strobe(b);
                Message::Leds(leds)
            })
            .into(),
            Checkbox::new(self.leds.flashlight(), "Flashlight", move |b| {
                let mut leds = leds;
                leds.set_flashlight(b);
                Message::Leds(leds)
            })
            .into(),
        ])
        .spacing(10)
        .into();

        let resolution = self.resolution;
        let r = |a, b| Radio::new(a, b, Some(resolution), Message::Resolution).into();
        let res_ctrl = Column::with_children(vec![
            title("Resolution"),
            r(Resolution::R320x240, "320x240"),
            r(Resolution::R160x120, "160x120"),
            r(Resolution::R80x60, "80x60"),
            r(Resolution::R40x30, "40x30"),
        ])
        .spacing(10)
        .into();

        let denoise_ctrl = Checkbox::new(self.denoise, "Denoise", Message::Denoise).into();

        let edge_ctrl = Column::with_children(vec![
            title("Edge smoothing"),
            self.edge_smoothing
                .render(
                    Message::EdgeSmoothing,
                    format!("0x{:x}", self.edge_smoothing.value as u8),
                )
                .into(),
        ])
        .spacing(10)
        .into();

        let color_ctrl = Column::with_children(vec![
            title("Color interpolation threshold"),
            self.color_interpolation_threshold
                .render(
                    Message::ColorInterpolationThreshold,
                    format!("0x{:x}", self.color_interpolation_threshold.value as u8),
                )
                .into(),
        ])
        .spacing(10)
        .into();

        let white_ctrl = Column::with_children(vec![
            title("White pixel threshold"),
            self.white_threshold
                .render(
                    Message::WhiteThreshold,
                    format!("0x{:x}", self.white_threshold.value as u8),
                )
                .into(),
        ])
        .spacing(10)
        .into();

        let update_ctrl = Column::with_children(vec![
            title("Buffer update time"),
            self.buffer_update_time
                .render(
                    Message::UpdateTime,
                    format!("0x{:x}", self.buffer_update_time.value as u8),
                )
                .into(),
        ])
        .spacing(10)
        .into();

        let exposure_ctrl = Column::with_children(vec![
            title("Exposure"),
            Checkbox::new(self.max_exposure, "Max", Message::MaxExposure).into(),
            self.exposure
                .render(
                    Message::Exposure,
                    format!("{} µs", self.exposure.value as u32),
                )
                .into(),
        ])
        .spacing(10)
        .into();

        let depth_ctrl = Text::new(format!(
            "{},{}: {}",
            self.depth.0, self.depth.1, self.depth.2
        ))
        .into();

        Container::new(
            Column::with_children(vec![
                general_ctrl,
                leds_ctrl,
                res_ctrl,
                denoise_ctrl,
                edge_ctrl,
                white_ctrl,
                color_ctrl,
                exposure_ctrl,
                update_ctrl,
                depth_ctrl,
            ])
            .spacing(15),
        )
        .max_width(300)
        .style(StyleSheet)
        .padding(10)
        .into()
    }
}

struct StyleSheet;

impl container::StyleSheet for StyleSheet {
    fn style(&self) -> container::Style {
        container::Style {
            text_color: Some(Color::BLACK),
            border_color: Color::BLACK,
            background: Some(Background::Color(Color::WHITE)),
            border_radius: 10,
            border_width: 3,
        }
    }
}

struct Sliderf32 {
    value: f32,
    max: f32,
    state: slider::State,
}

impl Sliderf32 {
    fn new(value: f32, max: f32) -> Self {
        Self {
            value,
            max,
            state: slider::State::new(),
        }
    }

    fn percent(&self) -> u32 {
        (self.value / self.max * 100.) as u32
    }

    fn render(
        &mut self,
        variant: impl 'static + Fn(f32) -> Message,
        display: String,
    ) -> Row<Message, Renderer> {
        Row::with_children(vec![
            Slider::new(&mut self.state, (0.)..=(self.max), self.value, variant).into(),
            Text::new(display)
                .vertical_alignment(VerticalAlignment::Center)
                .into(),
        ])
        .spacing(10)
    }
}
