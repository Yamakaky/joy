use enigo::{Key, MouseButton};
use enum_map::{Enum, EnumMap};
use hid_gamepad_types::JoyKey;
use std::{collections::HashMap, fmt::Debug, time::Duration};
use std::{convert::TryInto, time::Instant};

use crate::ClickType;

#[derive(Debug, Copy, Clone)]
pub enum Action {
    Layer(u8, bool),
    Ext(ExtAction),
}

#[derive(Debug, Copy, Clone)]
pub enum ExtAction {
    KeyPress(Key, ClickType),
    MousePress(MouseButton, ClickType),
    GyroOn(ClickType),
    GyroOff(ClickType),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum KeyStatus {
    Down,
    Up,
    Hold,
    DoubleUp,
    DoubleDown,
}

impl KeyStatus {
    pub fn is_down(self) -> bool {
        match self {
            KeyStatus::Down | KeyStatus::DoubleDown | KeyStatus::Hold => true,
            KeyStatus::Up | KeyStatus::DoubleUp => false,
        }
    }

    pub fn is_up(self) -> bool {
        !self.is_down()
    }
}

impl Default for KeyStatus {
    fn default() -> Self {
        KeyStatus::Up
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Layer {
    pub on_down: Option<Action>,
    pub on_up: Option<Action>,

    pub on_click: Option<Action>,
    pub on_double_click: Option<Action>,
    pub on_hold_down: Option<Action>,
    pub on_hold_up: Option<Action>,
}

impl Layer {
    fn is_good(&self) -> bool {
        self.on_down.is_some()
            || self.on_up.is_some()
            || self.on_click.is_some()
            || self.on_hold_down.is_some()
            || self.on_hold_up.is_some()
            || self.on_double_click.is_some()
    }

    fn is_simple_click(&self) -> bool {
        self.on_hold_down.is_none() && self.on_hold_up.is_none() && self.on_double_click.is_none()
    }
}

impl Default for Layer {
    fn default() -> Self {
        Layer {
            on_click: None,
            on_double_click: None,
            on_down: None,
            on_up: None,
            on_hold_down: None,
            on_hold_up: None,
        }
    }
}

#[derive(Debug, Clone)]
struct KeyState {
    status: KeyStatus,
    last_update: Instant,
}

impl Default for KeyState {
    fn default() -> Self {
        KeyState {
            status: KeyStatus::Up,
            last_update: Instant::now(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Enum)]
pub enum VirtualKey {
    LUp,
    LDown,
    LLeft,
    LRight,
    LRing,
    RUp,
    RDown,
    RLeft,
    RRight,
    RRing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapKey {
    Physical(JoyKey),
    Virtual(VirtualKey),
}

impl MapKey {
    pub fn to_layer(self) -> u8 {
        <Self as Enum<()>>::to_usize(self).try_into().unwrap()
    }
}

const JOYKEY_SIZE: usize = <JoyKey as Enum<()>>::POSSIBLE_VALUES;
const VIRTKEY_SIZE: usize = <VirtualKey as Enum<()>>::POSSIBLE_VALUES;
const MAP_KEY_SIZE: usize = JOYKEY_SIZE + VIRTKEY_SIZE;

impl<V: Default> Enum<V> for MapKey {
    type Array = [V; MAP_KEY_SIZE];

    const POSSIBLE_VALUES: usize = MAP_KEY_SIZE;

    fn slice(array: &Self::Array) -> &[V] {
        array
    }

    fn slice_mut(array: &mut Self::Array) -> &mut [V] {
        array
    }

    fn from_usize(value: usize) -> Self {
        if value < JOYKEY_SIZE {
            <JoyKey as Enum<()>>::from_usize(value).into()
        } else if value < MAP_KEY_SIZE {
            <VirtualKey as Enum<()>>::from_usize(value - JOYKEY_SIZE).into()
        } else {
            unimplemented!()
        }
    }

    fn to_usize(self) -> usize {
        match self {
            MapKey::Physical(p) => <JoyKey as Enum<()>>::to_usize(p),
            MapKey::Virtual(v) => <VirtualKey as Enum<()>>::to_usize(v) + JOYKEY_SIZE,
        }
    }

    fn from_function<F: FnMut(Self) -> V>(mut f: F) -> Self::Array {
        let mut out = Self::Array::default();
        for (i, out) in out.iter_mut().enumerate() {
            *out = f(<Self as Enum<V>>::from_usize(i));
        }
        out
    }
}

impl From<JoyKey> for MapKey {
    fn from(k: JoyKey) -> Self {
        MapKey::Physical(k)
    }
}

impl From<VirtualKey> for MapKey {
    fn from(k: VirtualKey) -> Self {
        MapKey::Virtual(k)
    }
}

#[derive(Debug, Clone)]
pub struct Buttons {
    bindings: EnumMap<MapKey, HashMap<u8, Layer>>,
    state: EnumMap<MapKey, KeyState>,
    current_layers: Vec<u8>,

    ext_actions: Vec<ExtAction>,

    pub hold_delay: Duration,
    pub double_click_interval: Duration,
}

impl Buttons {
    pub fn new() -> Self {
        Buttons {
            bindings: EnumMap::new(),
            state: EnumMap::new(),
            current_layers: vec![0],
            ext_actions: Vec::new(),
            hold_delay: Duration::from_millis(100),
            double_click_interval: Duration::from_millis(200),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn get(&mut self, key: MapKey, layer: u8) -> &mut Layer {
        self.bindings[key].entry(layer).or_default()
    }

    pub fn tick(&mut self, now: Instant) -> &mut Vec<ExtAction> {
        for key in (0..<MapKey as Enum<KeyStatus>>::POSSIBLE_VALUES)
            .map(<MapKey as Enum<KeyStatus>>::from_usize)
        {
            let binding = self.find_binding(key);
            match self.state[key].status {
                KeyStatus::Down => {
                    if let Some(ref hold_down) = binding.on_hold_down {
                        if now.duration_since(self.state[key].last_update) >= self.hold_delay {
                            Self::action(
                                hold_down,
                                &mut self.current_layers,
                                &mut self.ext_actions,
                            );
                            self.state[key].status = KeyStatus::Hold;
                        }
                    }
                }
                KeyStatus::DoubleUp => {
                    if now.duration_since(self.state[key].last_update) >= self.double_click_interval
                    {
                        Self::maybe_click(
                            &binding,
                            &mut self.current_layers,
                            &mut self.ext_actions,
                        );
                        self.state[key].status = KeyStatus::Up;
                    }
                }
                _ => (),
            }
        }
        &mut self.ext_actions
    }

    pub fn key_down<K: Into<MapKey>>(&mut self, key: K, now: Instant) {
        let key = key.into();
        if self.state[key].status.is_down() {
            return;
        }
        let binding = self.find_binding(key);
        if let Some(ref down) = binding.on_down {
            Self::action(down, &mut self.current_layers, &mut self.ext_actions);
        }
        if binding.is_simple_click() {
            Self::maybe_click(&binding, &mut self.current_layers, &mut self.ext_actions);
        }
        self.state[key].status = match self.state[key].status {
            KeyStatus::DoubleUp
                if now.duration_since(self.state[key].last_update) < self.double_click_interval =>
            {
                KeyStatus::DoubleDown
            }
            KeyStatus::Up => KeyStatus::Down,
            _ => unreachable!(),
        };
        self.state[key].last_update = now;
    }

    pub fn key_up<K: Into<MapKey>>(&mut self, key: K, now: Instant) {
        let key = key.into();
        if self.state[key].status.is_up() {
            return;
        }
        let binding = self.find_binding(key);
        if let Some(ref up) = binding.on_up {
            Self::action(up, &mut self.current_layers, &mut self.ext_actions);
        }
        let mut new_status = KeyStatus::Up;
        if !binding.is_simple_click() {
            if binding.on_hold_up.is_none()
                || now.duration_since(self.state[key].last_update) < self.hold_delay
            {
                if let Some(ref double) = binding.on_double_click {
                    match self.state[key].status {
                        KeyStatus::DoubleDown => {
                            Self::action(double, &mut self.current_layers, &mut self.ext_actions);
                            new_status = KeyStatus::Up;
                        }
                        KeyStatus::Down => {
                            new_status = KeyStatus::DoubleUp;
                        }
                        _ => unreachable!(),
                    }
                } else {
                    Self::maybe_click(&binding, &mut self.current_layers, &mut self.ext_actions);
                }
            } else if let Some(ref hold_up) = binding.on_hold_up {
                Self::action(hold_up, &mut self.current_layers, &mut self.ext_actions);
            }
        }
        self.state[key].status = new_status;
        self.state[key].last_update = now;
    }

    pub fn key<K: Into<MapKey>>(&mut self, key: K, pressed: bool, now: Instant) {
        let key = key.into();
        if pressed {
            self.key_down(key, now);
        } else {
            self.key_up(key, now);
        }
    }

    fn maybe_click(
        binding: &Layer,
        current_layers: &mut Vec<u8>,
        ext_actions: &mut Vec<ExtAction>,
    ) {
        if let Some(ref click) = binding.on_click {
            Self::action(click, current_layers, ext_actions);
        }
    }

    fn find_binding(&self, key: MapKey) -> Layer {
        let layers = &self.bindings[key];
        for i in self.current_layers.iter().rev() {
            if let Some(layer) = layers.get(i) {
                if layer.is_good() {
                    return *layer;
                }
            }
        }
        Layer::default()
    }

    fn action(action: &Action, current_layers: &mut Vec<u8>, ext_actions: &mut Vec<ExtAction>) {
        match *action {
            Action::Layer(l, true) => {
                if current_layers.contains(&l) {
                    current_layers.retain(|x| *x != l);
                }
                current_layers.push(l);
            }
            Action::Layer(l, false) => {
                current_layers.retain(|x| *x != l);
            }
            Action::Ext(action) => ext_actions.push(action),
        }
    }
}
