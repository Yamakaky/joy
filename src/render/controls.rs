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
    resolution: Resolution,
    edge_smoothing: f32,
    edge_state: slider::State,
    buffer_update_time: f32,
    buffer_update_time_state: slider::State,
    depth: (u32, u32, u8),
}

#[derive(Copy, Clone, Debug)]
pub enum Message {
    Leds(Leds),
    Resolution(Resolution),
    EdgeSmoothing(f32),
    UpdateTime(f32),
    Depth(u32, u32, u8),
}

impl Controls {
    pub fn new(thread_contact: mpsc::Sender<JoyconCmd>) -> Controls {
        let mut leds = Leds(0);
        leds.set_disable_far_narrow12(true);
        Controls {
            thread_contact,
            leds,
            resolution: Resolution::R160x120,
            edge_smoothing: 0x23 as f32,
            edge_state: slider::State::new(),
            buffer_update_time: 0x23 as f32,
            buffer_update_time_state: slider::State::new(),
            depth: (0, 0, 0),
        }
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
            Message::EdgeSmoothing(threshold) => {
                self.edge_smoothing = threshold;
                self.thread_contact
                    .send(JoyconCmd::SetRegister(Register::edge_smoothing_threshold(
                        threshold as u8,
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

        let leds = self.leds;
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
        ])
        .spacing(10);

        let r = |a, b| Radio::new(a, b, Some(self.resolution), Message::Resolution).into();
        let res_ctrl = Column::with_children(vec![
            title("Resolution"),
            r(Resolution::R320x240, "320x240"),
            r(Resolution::R160x120, "160x120"),
            r(Resolution::R80x60, "80x60"),
            r(Resolution::R40x30, "40x30"),
        ])
        .spacing(10);

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
        .spacing(10);

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
        .spacing(10);

        let depth_ctrl = Text::new(format!(
            "{},{}: {}",
            self.depth.0, self.depth.1, self.depth.2
        ));

        Container::new(
            Column::with_children(vec![
                leds_ctrl.into(),
                res_ctrl.into(),
                edge_ctrl.into(),
                update_ctrl.into(),
                depth_ctrl.into(),
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
