use enigo::Key;

use crate::{
    mapping::{Action, Buttons},
    ExtAction,
};

pub fn parse_file(content: &str, mapping: &mut Buttons<ExtAction>) {
    for line in content.lines() {
        let mut tokens = line.split_ascii_whitespace();
        let (k, v) = (tokens.next().unwrap(), tokens.next().unwrap());

        let mut combined = k.split(",");
        let (k1, k2) = (combined.next().unwrap(), combined.next());
        dbg!(k1, k2);
        let k1 = k1.parse().unwrap();
        let k2 = k2.map(|k| k.parse().unwrap());
        if let Some(k2) = k2 {
            let toggle = mapping.get(k1, 0);
            toggle.on_hold_down = Some(Action::Layer(k1 as u8, true));
            toggle.on_hold_up = Some(Action::Layer(k1 as u8, false));
            mapping.get(k2, k1 as u8).on_click = Some(Action::Ext(ExtAction::KeyPress(
                Key::Layout(v.chars().next().unwrap()),
                None,
            )));
        } else {
            mapping.get(k1, 0).on_click = Some(Action::Ext(ExtAction::KeyPress(
                Key::Layout(v.chars().next().unwrap()),
                None,
            )));
        }
    }
}
