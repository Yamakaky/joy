use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::{
        complete::{newline, not_line_ending, satisfy, space0, space1},
        is_alphanumeric,
    },
    combinator::{all_consuming, opt, value},
    error::context,
    multi::{separated_list0, separated_list1},
    IResult,
};

use crate::{
    mapping::{Action, Buttons, JoyKey, Layer},
    ClickType, ExtAction,
};

fn map_key(layer: &mut Layer<ExtAction>, actions: &Vec<JSMAction>) {
    use ActionModifier::*;
    use EventModifier::*;

    let mut first = true;
    for action in actions {
        match (
            action
                .event_mod
                .unwrap_or_else(|| if first { Tap } else { Hold }),
            action.action_mod,
        ) {
            (Tap, None) => {
                layer.on_click = Some(Action::Ext((action.action, ClickType::Click).into()));
            }
            (Tap, Some(Toggle)) => {
                layer.on_click = Some(Action::Ext((action.action, ClickType::Toggle).into()));
            }
            (Tap, Some(Instant)) => {
                layer.on_click = Some(Action::Ext((action.action, ClickType::Click).into()));
            }
            (Hold, None) => {
                layer.on_hold_down = Some(Action::Ext((action.action, ClickType::Press).into()));
                layer.on_hold_up = Some(Action::Ext((action.action, ClickType::Release).into()));
            }
            (Hold, Some(Toggle)) => {
                layer.on_hold_down = Some(Action::Ext((action.action, ClickType::Toggle).into()));
            }
            (Hold, Some(Instant)) => {
                layer.on_hold_down = Some(Action::Ext((action.action, ClickType::Click).into()));
            }
            (Start, None) => {
                layer.on_down = Some(Action::Ext((action.action, ClickType::Press).into()));
                layer.on_up = Some(Action::Ext((action.action, ClickType::Release).into()));
            }
            (Start, Some(Toggle)) => {
                layer.on_down = Some(Action::Ext((action.action, ClickType::Toggle).into()));
            }
            (Start, Some(Instant)) => {
                layer.on_down = Some(Action::Ext((action.action, ClickType::Click).into()));
            }
            (Release, None) => unreachable!(),
            (Release, Some(Toggle)) => {
                layer.on_up = Some(Action::Ext((action.action, ClickType::Toggle).into()));
            }
            (Release, Some(Instant)) => {
                layer.on_up = Some(Action::Ext((action.action, ClickType::Click).into()));
            }
            (Turbo, _) => unimplemented!(),
        }
        first = false;
    }
}

pub fn parse_file<'a>(content: &'a str, mapping: &mut Buttons<ExtAction>) -> IResult<&'a str, ()> {
    for cmd in jsm_parse(content)?.1 {
        match cmd {
            Cmd::Map(Key::Simple(key), ref actions) => map_key(mapping.get(key, 0), actions),

            Cmd::Map(Key::Chorded(k1, k2), ref actions) => {
                mapping.get(k1, 0).on_down = Some(Action::Layer(k1 as u8, true));
                mapping.get(k1, 0).on_up = Some(Action::Layer(k1 as u8, false));
                map_key(mapping.get(k2, k1 as u8), actions);
            }
            Cmd::Map(Key::Simul(_k1, _k2), ref _actions) => unimplemented!(),
        }
    }
    Ok(("", ()))
}

#[derive(Debug, Copy, Clone)]
pub enum ActionModifier {
    Toggle,
    Instant,
}

#[derive(Debug, Copy, Clone)]
pub enum EventModifier {
    Tap,
    Hold,
    Start,
    Release,
    Turbo,
}

#[derive(Debug, Copy, Clone)]
pub struct JSMAction {
    pub action_mod: Option<ActionModifier>,
    pub event_mod: Option<EventModifier>,
    pub action: ActionType,
}

#[derive(Debug, Copy, Clone)]
pub enum ActionType {
    Key(enigo::Key),
}

impl From<(ActionType, ClickType)> for ExtAction {
    fn from((a, b): (ActionType, ClickType)) -> Self {
        match a {
            ActionType::Key(k) => ExtAction::KeyPress(k, b),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Key {
    Simple(JoyKey),
    Simul(JoyKey, JoyKey),
    Chorded(JoyKey, JoyKey),
}

#[derive(Debug, Clone)]
pub enum Cmd {
    Map(Key, Vec<JSMAction>),
}

fn keys(input: &str) -> IResult<&str, Key> {
    fn simple(input: &str) -> IResult<&str, Key> {
        joykey(input).map(|(i, k)| (i, Key::Simple(k)))
    }
    fn simul(input: &str) -> IResult<&str, Key> {
        let (input, k1) = joykey(input)?;
        let (input, _) = space0(input)?;
        let (input, _) = tag("+")(input)?;
        let (input, _) = space0(input)?;
        let (input, k2) = joykey(input)?;
        Ok((input, Key::Simul(k1, k2)))
    }
    fn chorded(input: &str) -> IResult<&str, Key> {
        dbg!(input);
        let (input, k1) = joykey(input)?;
        let (input, _) = space0(input)?;
        let (input, _) = tag(",")(input)?;
        let (input, _) = space0(input)?;
        let (input, k2) = joykey(input)?;
        Ok((input, Key::Chorded(k1, k2)))
    }
    alt((simul, chorded, simple))(input)
}

fn action(input: &str) -> IResult<&str, JSMAction> {
    let (input, action_mod) = opt(alt((
        value(ActionModifier::Toggle, tag_no_case("^")),
        value(ActionModifier::Instant, tag_no_case("!")),
    )))(input)?;
    let (input, key) = keyboardkey(input)?;
    let (input, event_mod) = opt(alt((
        value(EventModifier::Tap, tag_no_case("'")),
        value(EventModifier::Hold, tag_no_case("_")),
        value(EventModifier::Start, tag_no_case("\\")),
        value(EventModifier::Release, tag_no_case("/")),
        value(EventModifier::Turbo, tag_no_case("+")),
    )))(input)?;
    Ok((
        input,
        JSMAction {
            action_mod,
            event_mod,
            action: ActionType::Key(key),
        },
    ))
}

fn binding(input: &str) -> IResult<&str, Cmd> {
    let (input, key) = keys(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag("=")(input)?;
    let (input, _) = space0(input)?;
    let (input, actions) = separated_list1(space1, action)(input)?;
    Ok((input, Cmd::Map(key, actions)))
}

fn cmd(input: &str) -> IResult<&str, Cmd> {
    binding(input)
}

fn comment(input: &str) -> IResult<&str, ()> {
    let (input, _) = tag("#")(input)?;
    let (input, _) = not_line_ending(input)?;
    Ok((input, ()))
}

fn line(input: &str) -> IResult<&str, Option<Cmd>> {
    let (input, cmd) = opt(cmd)(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = opt(comment)(input)?;
    Ok((input, cmd))
}

pub fn jsm_parse(input: &str) -> IResult<&str, Vec<Cmd>> {
    let (input, cmds) =
        all_consuming(separated_list0(newline, context("parse line", line)))(input)?;
    Ok((input, cmds.into_iter().flat_map(|x| x).collect()))
}

fn joykey(input: &str) -> IResult<&str, JoyKey> {
    alt((
        alt((
            value(JoyKey::Up, tag_no_case("Up")),
            value(JoyKey::Down, tag_no_case("Down")),
            value(JoyKey::Left, tag_no_case("Left")),
            value(JoyKey::Right, tag_no_case("Right")),
            value(JoyKey::LUp, tag_no_case("LUp")),
            value(JoyKey::LDown, tag_no_case("LDown")),
            value(JoyKey::LLeft, tag_no_case("LLeft")),
            value(JoyKey::LRight, tag_no_case("LRight")),
            value(JoyKey::RUp, tag_no_case("RUp")),
            value(JoyKey::RDown, tag_no_case("RDown")),
            value(JoyKey::RLeft, tag_no_case("RLeft")),
            value(JoyKey::RRight, tag_no_case("RRight")),
            value(JoyKey::N, tag_no_case("N")),
            value(JoyKey::S, tag_no_case("S")),
            value(JoyKey::E, tag_no_case("E")),
        )),
        alt((
            value(JoyKey::W, tag_no_case("W")),
            value(JoyKey::L, tag_no_case("L")),
            value(JoyKey::R, tag_no_case("R")),
            value(JoyKey::ZL, tag_no_case("ZL")),
            value(JoyKey::ZR, tag_no_case("ZR")),
            value(JoyKey::SL, tag_no_case("SL")),
            value(JoyKey::SR, tag_no_case("SR")),
            value(JoyKey::L3, tag_no_case("L3")),
            value(JoyKey::R3, tag_no_case("R3")),
            value(JoyKey::Minus, tag_no_case("Minus")),
            value(JoyKey::Plus, tag_no_case("Plus")),
            value(JoyKey::Capture, tag_no_case("Capture")),
            value(JoyKey::Home, tag_no_case("Home")),
        )),
    ))(input)
}

fn keyboardkey(input: &str) -> IResult<&str, enigo::Key> {
    use enigo::Key::*;
    let char_parse =
        |input| satisfy(|c| is_alphanumeric(c as u8))(input).map(|(i, x)| (i, Layout(x)));
    alt((
        value(Alt, tag_no_case("alt")),
        value(Backspace, tag_no_case("backspace")),
        char_parse,
    ))(input)
}
