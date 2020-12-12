use enum_map::{Enum, EnumMap};
use std::{collections::HashMap, fmt::Debug, time::Duration};
use std::{str::FromStr, time::Instant};

#[derive(Debug, Copy, Clone)]
pub enum Action<ExtAction> {
    Layer(u8, bool),
    Ext(ExtAction),
}

#[derive(Enum, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum JoyKey {
    Up,
    Down,
    Left,
    Right,
    N,
    S,
    E,
    W,
    L,
    R,
    ZL,
    ZR,
    SL,
    SR,
    L3,
    R3,
    Minus,
    Plus,
    Capture,
    Home,
}

impl FromStr for JoyKey {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use JoyKey::*;
        Ok(match s {
            "Up" => Up,
            "Down" => Down,
            "Left" => Left,
            "Right" => Right,
            "N" => N,
            "S" => S,
            "E" => E,
            "W" => W,
            "L" => L,
            "R" => R,
            "ZL" => ZL,
            "ZR" => ZR,
            "SL" => SL,
            "SR" => SR,
            "L3" => L3,
            "R3" => R3,
            "Minus" => Minus,
            "Plus" => Plus,
            "Capture" => Capture,
            "Home" => Home,
            _ => unreachable!(),
        })
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum KeyStatus {
    Down,
    Up,
    Hold,
    DoubleUp,
    DoubleDown,
}

#[derive(Debug, Copy, Clone)]
pub struct Layer<Ext> {
    pub on_down: Option<Action<Ext>>,
    pub on_up: Option<Action<Ext>>,

    pub on_click: Option<Action<Ext>>,
    pub on_double_click: Option<Action<Ext>>,
    pub on_hold_down: Option<Action<Ext>>,
    pub on_hold_up: Option<Action<Ext>>,
}

impl<Ext> Layer<Ext> {
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

impl<Ext> Default for Layer<Ext> {
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

#[derive(Debug, Clone)]
pub struct Buttons<ExtAction> {
    bindings: EnumMap<JoyKey, HashMap<u8, Layer<ExtAction>>>,
    state: EnumMap<JoyKey, KeyState>,
    current_layers: Vec<u8>,

    ext_actions: Vec<ExtAction>,

    pub hold_delay: Duration,
    pub double_click_interval: Duration,
}

impl<Ext: Copy> Buttons<Ext> {
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

    pub fn get(&mut self, key: JoyKey, layer: u8) -> &mut Layer<Ext> {
        self.bindings[key].entry(layer).or_default()
    }

    pub fn tick(&mut self, now: Instant) -> &mut Vec<Ext> {
        for key in (0..<JoyKey as Enum<KeyStatus>>::POSSIBLE_VALUES)
            .map(<JoyKey as Enum<KeyStatus>>::from_usize)
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

    pub fn key_down(&mut self, key: JoyKey, now: Instant) {
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

    pub fn key_up(&mut self, key: JoyKey, now: Instant) {
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

    fn maybe_click(binding: &Layer<Ext>, current_layers: &mut Vec<u8>, ext_actions: &mut Vec<Ext>) {
        if let Some(ref click) = binding.on_click {
            Self::action(click, current_layers, ext_actions);
        }
    }

    fn find_binding(&self, key: JoyKey) -> Layer<Ext> {
        let layers = &self.bindings[key];
        for i in self.current_layers.iter().rev() {
            if let Some(layer) = layers.get(&i) {
                if layer.is_good() {
                    return *layer;
                }
            }
        }
        Layer::default()
    }

    fn action(action: &Action<Ext>, current_layers: &mut Vec<u8>, ext_actions: &mut Vec<Ext>) {
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
