use super::JoyconCmd;
use iced_core::Background;
use iced_wgpu::{widget::container, Renderer};
use iced_winit::{
    slider, Checkbox, Color, Column, Command, Container, Element, HorizontalAlignment, Length,
    Program, Radio, Row, Slider, Text, VerticalAlignment,
};
use joycon_sys::mcu::ir::*;
use std::sync::mpsc;

pub struct Controls {
    thread_contact: mpsc::Sender<JoyconCmd>,
    leds: Leds,
    max_exposure: bool,
    exposure: f32,
    exposure_state: slider::State,
    far_int: f32,
    far_int_state: slider::State,
    near_int: f32,
    near_int_state: slider::State,
    resolution: Resolution,
    edge_smoothing: f32,
    edge_state: slider::State,
    buffer_update_time: f32,
    buffer_update_time_state: slider::State,
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
}

impl Controls {
    pub fn new(thread_contact: mpsc::Sender<JoyconCmd>) -> Controls {
        let mut leds = Leds(0);
        leds.set_disable_far_narrow12(true);
        Controls {
            thread_contact,
            leds,
            max_exposure: false,
            exposure: 200.,
            exposure_state: slider::State::new(),
            far_int: 0xf as f32,
            far_int_state: slider::State::new(),
            near_int: 0xf as f32,
            near_int_state: slider::State::new(),
            resolution: Resolution::R160x120,
            edge_smoothing: 0x23 as f32,
            edge_state: slider::State::new(),
            buffer_update_time: 0x23 as f32,
            buffer_update_time_state: slider::State::new(),
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
                self.far_int = far;
                self.near_int = near;
                self.thread_contact
                    .send(JoyconCmd::SetRegisters(Register::leds_intensity(
                        far as u8, near as u8,
                    )))
                    .unwrap();
            }
            Message::EdgeSmoothing(threshold) => {
                self.edge_smoothing = threshold;
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
                self.exposure = exposure;
                self.thread_contact
                    .send(JoyconCmd::SetRegisters(Register::exposure_us(
                        exposure as u32,
                    )))
                    .unwrap();
            }
            Message::UpdateTime(time) => {
                self.buffer_update_time = time;
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
        let (far_int, near_int) = (self.far_int, self.near_int);
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
            {
                Slider::new(
                    &mut self.far_int_state,
                    (0.)..=(15.),
                    self.far_int,
                    move |x| Message::Intensity(x, near_int),
                )
                .into()
            },
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
            Slider::new(
                &mut self.near_int_state,
                (0.)..=(15.),
                self.near_int,
                move |x| Message::Intensity(far_int, x),
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

        let edge_ctrl = Column::with_children(vec![
            title("Edge smoothing"),
            Row::with_children(vec![
                Slider::new(
                    &mut self.edge_state,
                    (0.)..=(255.),
                    self.edge_smoothing,
                    Message::EdgeSmoothing,
                )
                .into(),
                Text::new(format!("0x{:x}", self.edge_smoothing as u8))
                    .vertical_alignment(VerticalAlignment::Center)
                    .into(),
            ])
            .spacing(10)
            .into(),
        ])
        .spacing(10)
        .into();

        let update_ctrl = Column::with_children(vec![
            title("Buffer update time"),
            Row::with_children(vec![
                Slider::new(
                    &mut self.buffer_update_time_state,
                    (0.)..=(255.),
                    self.buffer_update_time,
                    Message::UpdateTime,
                )
                .into(),
                Text::new(format!("0x{:x}", self.buffer_update_time as u8))
                    .vertical_alignment(VerticalAlignment::Center)
                    .into(),
            ])
            .spacing(10)
            .into(),
        ])
        .spacing(10)
        .into();

        let exposure_ctrl = Column::with_children(vec![
            title("Exposure"),
            Checkbox::new(self.max_exposure, "Max", Message::MaxExposure).into(),
            Row::with_children(vec![
                Slider::new(
                    &mut self.exposure_state,
                    (0.)..=(600.),
                    self.exposure,
                    Message::Exposure,
                )
                .into(),
                Text::new(format!("{} µs", self.exposure as u32))
                    .vertical_alignment(VerticalAlignment::Center)
                    .into(),
            ])
            .spacing(10)
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
                edge_ctrl,
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
