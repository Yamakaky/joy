use hid_gamepad::sys::JoyKey;
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::{
        complete::{newline, not_line_ending, satisfy, space0, space1},
        is_alphanumeric,
    },
    combinator::{all_consuming, map, opt, value},
    error::context,
    multi::{separated_list0, separated_list1},
    IResult,
};

use crate::{
    mapping::{Action, Buttons, ExtAction, Layer, MapKey, VirtualKey},
    ClickType,
};

fn convert_action_mod(
    action: &JSMAction,
    action_mod: Option<ActionModifier>,
    default: ClickType,
) -> Option<Action> {
    if let ActionType::Special(s) = action.action {
        if s == SpecialKey::None {
            return None;
        }
    }
    let action_type = match action_mod {
        None => default,
        Some(ActionModifier::Toggle) => ClickType::Toggle,
        Some(ActionModifier::Instant) => ClickType::Click,
    };
    Some(Action::Ext((action.action, action_type).into()))
}

fn map_key(layer: &mut Layer, actions: &Vec<JSMAction>) {
    use EventModifier::*;

    let mut first = true;
    for action in actions {
        match (
            action.event_mod.unwrap_or_else(|| {
                if first {
                    if actions.len() == 1 {
                        Start
                    } else {
                        Tap
                    }
                } else {
                    Hold
                }
            }),
            action.action_mod,
        ) {
            (Tap, modifier) => {
                layer.on_click = convert_action_mod(action, modifier, ClickType::Click);
            }
            (Hold, modifier) => {
                layer.on_hold_down = convert_action_mod(action, modifier, ClickType::Press);
                if modifier.is_none() {
                    layer.on_hold_up = convert_action_mod(action, modifier, ClickType::Release);
                }
            }
            (Start, modifier) => {
                layer.on_down = convert_action_mod(action, modifier, ClickType::Press);
                if modifier.is_none() {
                    layer.on_up = convert_action_mod(action, modifier, ClickType::Release);
                }
            }
            (Release, None) => unreachable!(),
            (Release, modifier) => {
                layer.on_up = convert_action_mod(action, modifier, ClickType::Release);
            }
            (Turbo, _) => unimplemented!(),
        }
        first = false;
    }
}

pub fn parse_file<'a>(content: &'a str, mapping: &mut Buttons) -> IResult<&'a str, ()> {
    for cmd in jsm_parse(content)?.1 {
        match cmd {
            Cmd::Map(Key::Simple(key), ref actions) => map_key(mapping.get(key, 0), actions),

            Cmd::Map(Key::Chorded(k1, k2), ref actions) => {
                mapping.get(k1, 0).on_down = Some(Action::Layer(k1.to_layer(), true));
                mapping.get(k1, 0).on_up = Some(Action::Layer(k1.to_layer(), false));
                map_key(mapping.get(k2, k1.to_layer()), actions);
            }
            Cmd::Map(Key::Simul(_k1, _k2), ref _actions) => unimplemented!(),
            _ => unimplemented!(),
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
    Mouse(enigo::MouseButton),
    Special(SpecialKey),
}

impl From<(ActionType, ClickType)> for ExtAction {
    fn from((a, b): (ActionType, ClickType)) -> Self {
        match a {
            ActionType::Key(k) => ExtAction::KeyPress(k, b),
            ActionType::Mouse(k) => ExtAction::MousePress(k, b),
            ActionType::Special(SpecialKey::GyroOn) => ExtAction::GyroOn(b),
            ActionType::Special(SpecialKey::GyroOff) => ExtAction::GyroOff(b),
            ActionType::Special(SpecialKey::None) => unimplemented!(),
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Key {
    Simple(MapKey),
    Simul(MapKey, MapKey),
    Chorded(MapKey, MapKey),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SpecialKey {
    None,
    GyroOn,
    GyroOff,
    GyroInvertX(bool),
    GyroInvertY(bool),
    GyroTrackBall(bool),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TriggerMode {
    NoFull,
    NoSkip,
    NoSkipExclusive,
    MustSkip,
    MaySkip,
    MustSkipR,
    MaySkipR,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum StickMode {
    Aim,
    Flick,
    FlickOnly,
    RotateOnly,
    MouseRing,
    MouseArea,
    NoMouse,
    ScrollWheel,
}

#[derive(Debug, Copy, Clone)]
pub enum StickSetting {
    Sens(f64),
    Power(f64),
    InvertX,
    InvertY,
    AccelerationRate(f64),
    AccelerationCap(f64),
    Deadzone(f64),
    FullZone(f64),
}

#[derive(Debug, Copy, Clone)]
pub enum Setting {
    TriggerThreshold(f64),
    ZLMode(TriggerMode),
    ZRMode(TriggerMode),
    LeftStickMode(StickMode),
    RightStickMode(StickMode),
    StickSetting(StickSetting),
}

#[derive(Debug, Clone)]
pub enum Cmd {
    Map(Key, Vec<JSMAction>),
    Special(SpecialKey),
    Setting(Setting),
}

fn keys(input: &str) -> IResult<&str, Key> {
    fn simple(input: &str) -> IResult<&str, Key> {
        mapkey(input).map(|(i, k)| (i, Key::Simple(k)))
    }
    fn simul(input: &str) -> IResult<&str, Key> {
        let (input, k1) = mapkey(input)?;
        let (input, _) = space0(input)?;
        let (input, _) = tag("+")(input)?;
        let (input, _) = space0(input)?;
        let (input, k2) = mapkey(input)?;
        Ok((input, Key::Simul(k1, k2)))
    }
    fn chorded(input: &str) -> IResult<&str, Key> {
        let (input, k1) = mapkey(input)?;
        let (input, _) = space0(input)?;
        let (input, _) = tag(",")(input)?;
        let (input, _) = space0(input)?;
        let (input, k2) = mapkey(input)?;
        Ok((input, Key::Chorded(k1, k2)))
    }
    alt((simul, chorded, simple))(input)
}

fn action(input: &str) -> IResult<&str, JSMAction> {
    let (input, action_mod) = opt(alt((
        value(ActionModifier::Toggle, tag("^")),
        value(ActionModifier::Instant, tag("!")),
    )))(input)?;
    let (input, action) = alt((
        map(special, ActionType::Special),
        map(mousekey, ActionType::Mouse),
        map(keyboardkey, ActionType::Key),
    ))(input)?;
    let (input, event_mod) = opt(alt((
        value(EventModifier::Tap, tag("'")),
        value(EventModifier::Hold, tag("_")),
        value(EventModifier::Start, tag("\\")),
        value(EventModifier::Release, tag("/")),
        value(EventModifier::Turbo, tag("+")),
    )))(input)?;
    Ok((
        input,
        JSMAction {
            action_mod,
            event_mod,
            action,
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
    alt((binding, map(special, Cmd::Special)))(input)
}

fn comment(input: &str) -> IResult<&str, ()> {
    let (input, _) = tag("#")(input)?;
    let (input, _) = not_line_ending(input)?;
    Ok((input, ()))
}

fn line(input: &str) -> IResult<&str, Option<Cmd>> {
    let (input, _) = space0(input)?;
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

fn mapkey(input: &str) -> IResult<&str, MapKey> {
    alt((map(virtkey, MapKey::from), map(joykey, MapKey::from)))(input)
}

fn joykey(input: &str) -> IResult<&str, JoyKey> {
    alt((
        alt((
            value(JoyKey::Up, tag_no_case("Up")),
            value(JoyKey::Down, tag_no_case("Down")),
            value(JoyKey::Left, tag_no_case("Left")),
            value(JoyKey::Right, tag_no_case("Right")),
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

fn virtkey(input: &str) -> IResult<&str, VirtualKey> {
    alt((
        value(VirtualKey::LUp, tag_no_case("LUp")),
        value(VirtualKey::LDown, tag_no_case("LDown")),
        value(VirtualKey::LLeft, tag_no_case("LLeft")),
        value(VirtualKey::LRight, tag_no_case("LRight")),
        value(VirtualKey::LRing, tag_no_case("LRing")),
        value(VirtualKey::RUp, tag_no_case("RUp")),
        value(VirtualKey::RDown, tag_no_case("RDown")),
        value(VirtualKey::RLeft, tag_no_case("RLeft")),
        value(VirtualKey::RRight, tag_no_case("RRight")),
        value(VirtualKey::RRing, tag_no_case("RRing")),
    ))(input)
}

fn keyboardkey(input: &str) -> IResult<&str, enigo::Key> {
    use enigo::Key::*;
    let char_parse =
        |input| satisfy(|c| is_alphanumeric(c as u8))(input).map(|(i, x)| (i, Layout(x)));
    let key_parse = |key, tag| value(key, tag_no_case(tag));
    alt((
        alt((
            key_parse(Alt, "alt"),
            key_parse(Backspace, "backspace"),
            key_parse(CapsLock, "capslock"),
            key_parse(Control, "Control"),
            key_parse(Delete, "Delete"),
            key_parse(DownArrow, "down"),
            key_parse(End, "End"),
            key_parse(Escape, "Escape"),
            key_parse(F1, "F1"),
            key_parse(F10, "F10"),
            key_parse(F11, "F11"),
            key_parse(F12, "F12"),
            key_parse(F2, "F2"),
            key_parse(F3, "F3"),
            key_parse(F4, "F4"),
            key_parse(F5, "F5"),
        )),
        alt((
            key_parse(F6, "F6"),
            key_parse(F7, "F7"),
            key_parse(F8, "F8"),
            key_parse(F9, "F9"),
            key_parse(Home, "Home"),
            key_parse(LeftArrow, "left"),
            key_parse(Meta, "Meta"),
            key_parse(Option, "Option"),
            key_parse(PageDown, "PageDown"),
            key_parse(PageUp, "PageUp"),
            key_parse(Return, "Return"),
            key_parse(RightArrow, "right"),
            key_parse(Shift, "Shift"),
            key_parse(Space, "Space"),
            key_parse(Tab, "Tab"),
            key_parse(UpArrow, "up"),
            char_parse,
        )),
    ))(input)
}

fn mousekey(input: &str) -> IResult<&str, enigo::MouseButton> {
    use enigo::MouseButton::*;
    let key_parse = |key, tag| value(key, tag_no_case(tag));
    alt((
        key_parse(Left, "LMouse"),
        key_parse(Middle, "MMouse"),
        key_parse(Right, "RMouse"),
        key_parse(ScrollUp, "scrollup"),
        key_parse(ScrollDown, "scrolldown"),
        key_parse(ScrollLeft, "scrollleft"),
        key_parse(ScrollRight, "scrollright"),
    ))(input)
}

fn special(input: &str) -> IResult<&str, SpecialKey> {
    use SpecialKey::*;
    let parse = |key, tag| value(key, tag_no_case(tag));
    alt((
        parse(None, "none"),
        parse(GyroOn, "gyro_on"),
        parse(GyroOff, "gyro_off"),
        parse(GyroInvertX(true), "gyro_inv_x"),
        parse(GyroInvertY(true), "gyro_inv_y"),
        parse(GyroTrackBall(true), "gyro_trackball"),
    ))(input)
}
