use std::time::Duration;

use super::{settings::Settings, types::*};
use cgmath::Deg;
use hid_gamepad_types::JoyKey;
use nom::{
    branch::alt,
    character::{
        complete::{line_ending, not_line_ending, satisfy, space0, space1},
        is_alphanumeric,
    },
    combinator::{eof, map, opt, peek, value},
    multi::separated_list1,
    number::complete::float,
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::collect_separated_terminated,
    parser_ext::ParserExt,
    tag::complete::{tag, tag_no_case},
};

use crate::{
    mapping::{Action, Buttons, Layer, MapKey, VirtualKey},
    mouse::Mouse,
    ClickType,
};

type Input<'a> = &'a str;
type IRes<'a, O> = IResult<Input<'a>, O, ErrorTree<Input<'a>>>;

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

fn map_key(layer: &mut Layer, actions: &[JSMAction]) {
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

pub fn parse_file<'a>(
    content: &'a str,
    settings: &mut Settings,
    mapping: &mut Buttons,
    mouse: &mut Mouse,
) -> anyhow::Result<()> {
    for cmd in jsm_parse(content)? {
        match cmd {
            Cmd::Map(Key::Simple(key), ref actions) => map_key(mapping.get(key, 0), actions),

            Cmd::Map(Key::Chorded(k1, k2), ref actions) => {
                mapping.get(k1, 0).on_down = Some(Action::Layer(k1.to_layer(), true));
                mapping.get(k1, 0).on_up = Some(Action::Layer(k1.to_layer(), false));
                map_key(mapping.get(k2, k1.to_layer()), actions);
            }
            Cmd::Map(Key::Simul(_k1, _k2), ref _actions) => unimplemented!(),
            Cmd::Setting(setting) => settings.apply(setting),
            Cmd::Reset => {
                settings.reset();
                mapping.reset()
            }
            Cmd::RealWorldCalibration(c) => mouse.set_calibration(c),
            Cmd::InGameSens(c) => mouse.set_game_sens(c),
            Cmd::CounterOSMouseSpeed(c) => mouse.set_counter_os_speed(c),
            Cmd::Special(_) => unimplemented!(),
        }
    }
    Ok(())
}

fn keys(input: Input) -> IRes<'_, Key> {
    fn simple(input: Input) -> IRes<Key> {
        mapkey(input).map(|(i, k)| (i, Key::Simple(k)))
    }
    fn simul(input: Input) -> IRes<'_, Key> {
        let (input, k1) = mapkey(input)?;
        let (input, _) = space0(input)?;
        let (input, _) = tag("+")(input)?;
        let (input, _) = space0(input)?;
        let (input, k2) = mapkey(input)?;
        Ok((input, Key::Simul(k1, k2)))
    }
    fn chorded(input: Input) -> IRes<'_, Key> {
        let (input, k1) = mapkey(input)?;
        let (input, _) = space0(input)?;
        let (input, _) = tag(",")(input)?;
        let (input, _) = space0(input)?;
        let (input, k2) = mapkey(input)?;
        Ok((input, Key::Chorded(k1, k2)))
    }
    alt((simul, chorded, simple))(input)
}

fn action(input: Input) -> IRes<'_, JSMAction> {
    let (input, action_mod) = opt(alt((
        value(ActionModifier::Toggle, tag("^")),
        value(ActionModifier::Instant, tag("!")),
    )))
    .context("modifier")
    .parse(input)?;
    let (input, action) = alt((
        map(special, ActionType::Special),
        map(mousekey, ActionType::Mouse),
        map(keyboardkey, ActionType::Key),
    ))
    .context("action")
    .parse(input)?;
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

fn binding(input: Input) -> IRes<'_, Cmd> {
    let (input, key) = keys.context("parse keys").parse(input)?;
    let (input, _) = tag("=").delimited_by(space0).parse(input)?;
    let (input, actions) = separated_list1(space1, action)
        .context("parse actions")
        .parse(input)?;
    Ok((input, Cmd::Map(key, actions)))
}

fn setting(input: Input) -> IRes<'_, Setting> {
    alt((
        stick_mode,
        f64_setting("TRIGGER_THRESHOLD", Setting::TriggerThreshold),
        trigger_mode,
        gyro_setting,
        ring_mode,
        map(stick_setting, Setting::Stick),
    ))(input)
}

fn f64_setting<'a, Output>(
    tag: &'static str,
    value_map: impl Fn(f64) -> Output,
) -> impl FnMut(Input) -> IRes<'_, Output> {
    move |input| {
        let (input, _) = tag_no_case(tag)(input)?;
        let (input, _) = equal_with_space(input)?;
        let (input, val) = float(input)?;
        Ok((input, value_map(val as f64)))
    }
}

fn stick_setting(input: Input) -> IRes<'_, StickSetting> {
    alt((
        f64_setting("STICK_DEADZONE_INNER", StickSetting::Deadzone),
        f64_setting("STICK_DEADZONE_OUTER", |v| StickSetting::FullZone(1. - v)),
        f64_setting("STICK_SENS", |v| {
            StickSetting::Aim(AimStickSetting::Sens(v))
        }),
        f64_setting("STICK_POWER", |v| {
            StickSetting::Aim(AimStickSetting::Power(v))
        }),
        stick_axis,
        f64_setting("STICK_ACCELERATION_RATE", |v| {
            StickSetting::Aim(AimStickSetting::AccelerationRate(v))
        }),
        f64_setting("STICK_ACCELERATION_CAP", |v| {
            StickSetting::Aim(AimStickSetting::AccelerationCap(v))
        }),
        f64_setting("FLICK_TIME", |v| {
            StickSetting::Flick(FlickStickSetting::FlickTime(Duration::from_secs_f64(v)))
        }),
        f64_setting("FLICK_TIME_EXPONENT", |v| {
            StickSetting::Flick(FlickStickSetting::Exponent(v))
        }),
        f64_setting("FLICK_DEADZONE_ANGLE", |v| {
            StickSetting::Flick(FlickStickSetting::ForwardDeadzoneArc(Deg(v * 2.)))
        }),
        f64_setting("SCROLL_SENS", |v| {
            StickSetting::Scroll(ScrollStickSetting::Sens(Deg(v)))
        }),
    ))(input)
}

fn stick_axis(input: Input) -> IRes<'_, StickSetting> {
    let (input, tag) = alt((tag_no_case("STICK_AXIS_X"), tag_no_case("STICK_AXIS_Y")))(input)?;
    let (input, _) = equal_with_space(input)?;
    let (input, invert) = alt((
        value(false, tag_no_case("STANDARD")),
        value(true, tag_no_case("INVERTED")),
    ))(input)?;
    Ok((
        input,
        if tag == "STICK_AXIS_X" {
            StickSetting::Aim(AimStickSetting::InvertX(invert))
        } else {
            StickSetting::Aim(AimStickSetting::InvertY(invert))
        },
    ))
}

fn ring_mode(input: Input) -> IRes<'_, Setting> {
    let (input, tag) = alt((
        tag_no_case("LEFT_RING_MODE"),
        tag_no_case("RIGHT_RING_MODE"),
    ))(input)?;
    let (input, _) = equal_with_space(input)?;
    let (input, mode) = alt((
        value(RingMode::Inner, tag_no_case("INNER")),
        value(RingMode::Outer, tag_no_case("OUTER")),
    ))(input)?;
    Ok((
        input,
        if tag == "LEFT_RING_MODE" {
            Setting::LeftRingMode(mode)
        } else {
            Setting::RightRingMode(mode)
        },
    ))
}

fn gyro_setting(input: Input) -> IRes<'_, Setting> {
    map(
        alt((
            f64_setting("GYRO_SENS", GyroSetting::Sensitivity),
            f64_setting("MIN_GYRO_SENS", GyroSetting::MinSens),
            f64_setting("MIN_GYRO_THRESHOLD", GyroSetting::MinThreshold),
            f64_setting("MAX_GYRO_SENS", GyroSetting::MaxSens),
            f64_setting("MAX_GYRO_THRESHOLD", GyroSetting::MaxThreshold),
            gyro_space,
            f64_setting("GYRO_CUTOFF_SPEED", GyroSetting::CutoffSpeed),
            f64_setting("GYRO_CUTOFF_RECOVERY", GyroSetting::CutoffRecovery),
            f64_setting("GYRO_SMOOTH_THRESHOLD", GyroSetting::SmoothThreshold),
            f64_setting("GYRO_SMOOTH_TIME", |secs| {
                GyroSetting::SmoothTime(Duration::from_secs_f64(secs))
            }),
        )),
        Setting::Gyro,
    )(input)
}

fn gyro_space(input: Input) -> IRes<'_, GyroSetting> {
    let (input, _) = tag_no_case("GYRO_SPACE")(input)?;
    let (input, _) = equal_with_space(input)?;
    let (input, space) = alt((
        value(GyroSpace::Local, tag_no_case("LOCAL")),
        value(GyroSpace::WorldTurn, tag_no_case("WORLD_TURN")),
        value(GyroSpace::WorldLean, tag_no_case("WORLD_LEAN")),
        value(GyroSpace::PlayerTurn, tag_no_case("PLAYER_TURN")),
        value(GyroSpace::PlayerLean, tag_no_case("PLAYER_LEAN")),
    ))(input)?;
    Ok((input, GyroSetting::Space(space)))
}

fn stick_mode(input: Input) -> IRes<'_, Setting> {
    let (input, key) = alt((
        tag_no_case("LEFT_STICK_MODE"),
        tag_no_case("RIGHT_STICK_MODE"),
    ))(input)?;
    let (input, _) = equal_with_space(input)?;
    let (input, mode) = alt((
        value(StickMode::Aim, tag_no_case("AIM")),
        value(StickMode::Flick, tag_no_case("FLICK")),
        value(StickMode::FlickOnly, tag_no_case("FLICK_ONLY")),
        value(StickMode::MouseArea, tag_no_case("MOUSE_AREA")),
        value(StickMode::MouseRing, tag_no_case("MOUSE_RING")),
        value(StickMode::NoMouse, tag_no_case("NO_MOUSE")),
        value(StickMode::RotateOnly, tag_no_case("ROTATE_ONLY")),
        value(StickMode::ScrollWheel, tag_no_case("SCROLL_WHEEL")),
    ))(input)?;
    if key == "LEFT_STICK_MODE" {
        Ok((input, Setting::LeftStickMode(mode)))
    } else {
        Ok((input, Setting::RightStickMode(mode)))
    }
}

fn trigger_mode(input: Input) -> IRes<'_, Setting> {
    let (input, key) = alt((tag_no_case("ZL_MODE"), tag_no_case("ZR_MODE")))(input)?;
    let (input, _) = equal_with_space(input)?;
    let (input, mode) = alt((
        value(TriggerMode::MaySkip, tag_no_case("MAY_SKIP")),
        value(TriggerMode::MaySkipR, tag_no_case("MAY_SKIP_R")),
        value(TriggerMode::MustSkip, tag_no_case("MUST_SKIP")),
        value(TriggerMode::MustSkipR, tag_no_case("MUST_SKIP_R")),
        value(TriggerMode::NoFull, tag_no_case("NO_FULL")),
        value(TriggerMode::NoSkip, tag_no_case("NO_SKIP")),
        value(
            TriggerMode::NoSkipExclusive,
            tag_no_case("NO_SKIP_EXCLUSIVE"),
        ),
    ))(input)?;
    if key == "ZR_MODE" {
        Ok((input, Setting::ZRMode(mode)))
    } else {
        Ok((input, Setting::ZLMode(mode)))
    }
}

fn equal_with_space(input: Input) -> IRes<'_, ()> {
    let (input, _) = space0(input)?;
    let (input, _) = tag("=")(input)?;
    let (input, _) = space0(input)?;
    Ok((input, ()))
}

fn cmd(input: Input) -> IRes<'_, Cmd> {
    alt((
        binding.context("key binding"),
        map(special, Cmd::Special),
        map(setting, Cmd::Setting),
        value(Cmd::Reset, tag_no_case("RESET_MAPPINGS")),
        f64_setting("REAL_WORLD_CALIBRATION", Cmd::RealWorldCalibration),
        f64_setting("IN_GAME_SENS", Cmd::InGameSens),
        value(
            Cmd::CounterOSMouseSpeed(true),
            tag_no_case("COUNTER_OS_MOUSE_SPEED"),
        ),
    ))(input)
}

fn comment(input: Input) -> IRes<'_, ()> {
    let (input, _) = tag("#")(input)?;
    let (input, _) = not_line_ending(input)?;
    Ok((input, ()))
}

fn empty_line(input: Input) -> IRes<'_, Option<Cmd>> {
    let (input, _) = space0(input)?;
    let (input, _) = opt(comment)(input)?;
    peek(alt((line_ending, eof)))(input)?;
    Ok((input, None))
}

fn line(input: Input) -> IRes<'_, Option<Cmd>> {
    let (input, _) = space0(input)?;
    let (input, cmd) = cmd.context("command").parse(input)?;
    let (input, _) = empty_line(input)?;
    Ok((input, Some(cmd)))
}

pub fn jsm_parse(input: Input) -> Result<Vec<Cmd>, ErrorTree<Location>> {
    Ok(final_parser(
        collect_separated_terminated(alt((empty_line, line)).context("line"), line_ending, eof)
            .map(|cmds: Vec<_>| cmds.into_iter().flatten().collect()),
    )(input)?)
}

fn mapkey(input: Input) -> IRes<'_, MapKey> {
    alt((map(virtkey, MapKey::from), map(joykey, MapKey::from)))(input)
}

fn joykey(input: Input) -> IRes<'_, JoyKey> {
    alt((
        alt((
            value(JoyKey::Up, tag_no_case("Up")),
            value(JoyKey::Down, tag_no_case("Down")),
            value(JoyKey::Left, tag_no_case("Left")),
            value(JoyKey::Right, tag_no_case("Right")),
            value(JoyKey::ZL, tag_no_case("ZL")),
            value(JoyKey::ZR, tag_no_case("ZR")),
            value(JoyKey::SL, tag_no_case("SL")),
            value(JoyKey::SR, tag_no_case("SR")),
            value(JoyKey::L3, tag_no_case("L3")),
            value(JoyKey::R3, tag_no_case("R3")),
            value(JoyKey::N, tag_no_case("N")),
            value(JoyKey::S, tag_no_case("S")),
            value(JoyKey::E, tag_no_case("E")),
        )),
        alt((
            value(JoyKey::W, tag_no_case("W")),
            value(JoyKey::L, tag_no_case("L")),
            value(JoyKey::R, tag_no_case("R")),
            value(JoyKey::Minus, tag_no_case("-")),
            value(JoyKey::Plus, tag_no_case("+")),
            value(JoyKey::Minus, tag_no_case("Minus")),
            value(JoyKey::Plus, tag_no_case("Plus")),
            value(JoyKey::Capture, tag_no_case("Capture")),
            value(JoyKey::Home, tag_no_case("Home")),
        )),
    ))(input)
}

fn virtkey(input: Input) -> IRes<'_, VirtualKey> {
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

fn keyboardkey(input: Input) -> IRes<'_, enigo::Key> {
    use enigo::Key::*;
    let char_parse =
        |input| satisfy(|c| is_alphanumeric(c as u8))(input).map(|(i, x)| (i, Layout(x)));
    let key_parse = |key, tag| value(key, tag_no_case(tag));
    alt((
        alt((
            key_parse(Alt, "alt"),
            //TODO: proper lalt and ralt
            key_parse(Alt, "lalt"),
            key_parse(Alt, "ralt"),
            key_parse(Backspace, "backspace"),
            key_parse(CapsLock, "capslock"),
            key_parse(Control, "Control"),
            key_parse(Delete, "Delete"),
            key_parse(DownArrow, "down"),
            key_parse(End, "End"),
            key_parse(Escape, "Esc"),
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
            key_parse(Meta, "Windows"),
            key_parse(Meta, "lWindows"),
            key_parse(Meta, "rWindows"),
            key_parse(Option, "Option"),
            key_parse(PageDown, "PageDown"),
            key_parse(PageUp, "PageUp"),
            key_parse(Return, "Enter"),
            key_parse(RightArrow, "right"),
            key_parse(Shift, "Shift"),
            key_parse(Space, "Space"),
            key_parse(Tab, "Tab"),
            key_parse(UpArrow, "up"),
            char_parse,
        )),
    ))(input)
}

fn mousekey(input: Input) -> IRes<'_, enigo::MouseButton> {
    use enigo::MouseButton::*;
    let key_parse = |key, tag| value(key, tag_no_case(tag));
    alt((
        key_parse(Left, "LMouse"),
        key_parse(Middle, "MMouse"),
        key_parse(Right, "RMouse"),
        // TODO: fix https://github.com/enigo-rs/enigo/issues/110
        key_parse(Left, "BMouse"),
        key_parse(Left, "FMouse"),
        key_parse(ScrollUp, "scrollup"),
        key_parse(ScrollDown, "scrolldown"),
        key_parse(ScrollLeft, "scrollleft"),
        key_parse(ScrollRight, "scrollright"),
    ))(input)
}

fn special(input: Input) -> IRes<'_, SpecialKey> {
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
