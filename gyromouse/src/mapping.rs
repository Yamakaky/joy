use enum_map::{Enum, EnumMap};
use std::time::Duration;
use std::time::Instant;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Action {
    KeyPress(char, Option<bool>),
    Layer(u8, bool),
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum KeyStatus {
    Down,
    Up,
    Hold,
    DoubleUp,
    DoubleDown,
}

#[derive(Debug, Copy, Clone, Default)]
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
pub struct Buttons {
    bindings: EnumMap<JoyKey, EnumMap<u8, Layer>>,
    state: EnumMap<JoyKey, KeyState>,
    current_layers: Vec<u8>,

    pub hold_delay: Duration,
    pub double_click_interval: Duration,
}

impl Buttons {
    pub fn new() -> Buttons {
        Buttons {
            bindings: EnumMap::new(),
            state: EnumMap::new(),
            current_layers: Vec::new(),
            hold_delay: Duration::from_millis(100),
            double_click_interval: Duration::from_millis(200),
        }
    }

    pub fn set_binding(&mut self, key: JoyKey, layer: u8, binding: Layer) {
        self.bindings[key][layer] = binding;
    }

    pub fn tick(&mut self, now: Instant) {
        for key in (0..<JoyKey as Enum<KeyStatus>>::POSSIBLE_VALUES)
            .map(<JoyKey as Enum<KeyStatus>>::from_usize)
        {
            let binding = self.find_binding(key);
            match self.state[key].status {
                KeyStatus::Down => {
                    if let Some(ref hold_down) = binding.on_hold_down {
                        if now.duration_since(self.state[key].last_update) >= self.hold_delay {
                            Self::action(hold_down, &mut self.current_layers);
                            self.state[key].status = KeyStatus::Hold;
                        }
                    }
                }
                KeyStatus::DoubleUp => {
                    if now.duration_since(self.state[key].last_update) >= self.double_click_interval
                    {
                        Self::maybe_click(&binding, &mut self.current_layers);
                        self.state[key].status = KeyStatus::Up;
                    }
                }
                _ => (),
            }
        }
    }

    pub fn key_down(&mut self, key: JoyKey, now: Instant) {
        let binding = self.find_binding(key);
        if let Some(ref down) = binding.on_down {
            Self::action(down, &mut self.current_layers);
        }
        if binding.is_simple_click() {
            Self::maybe_click(&binding, &mut self.current_layers);
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
            Self::action(up, &mut self.current_layers);
        }
        let mut new_status = KeyStatus::Up;
        if !binding.is_simple_click() {
            if binding.on_hold_up.is_none()
                || now.duration_since(self.state[key].last_update) < self.hold_delay
            {
                if let Some(ref double) = binding.on_double_click {
                    match self.state[key].status {
                        KeyStatus::DoubleDown => {
                            Self::action(double, &mut self.current_layers);
                            new_status = KeyStatus::Up;
                        }
                        KeyStatus::Down => {
                            new_status = KeyStatus::DoubleUp;
                        }
                        _ => unreachable!(),
                    }
                } else {
                    Self::maybe_click(&binding, &mut self.current_layers);
                }
            } else if let Some(ref hold_up) = binding.on_hold_up {
                Self::action(hold_up, &mut self.current_layers);
            }
        }
        self.state[key].status = new_status;
        self.state[key].last_update = now;
    }

    fn maybe_click(binding: &Layer, current_layers: &mut Vec<u8>) {
        if let Some(ref click) = binding.on_click {
            Self::action(click, current_layers);
        }
    }

    fn find_binding(&self, key: JoyKey) -> Layer {
        let layers = &self.bindings[key];
        for i in &self.current_layers {
            if layers[*i].is_good() {
                return layers[*i];
            }
        }
        layers[0]
    }

    fn action(action: &Action, current_layers: &mut Vec<u8>) {
        match *action {
            Action::KeyPress(c, None) => println!("click {}", c),
            Action::KeyPress(c, Some(true)) => println!("down {}", c),
            Action::KeyPress(c, Some(false)) => println!("up {}", c),
            Action::Layer(l, true) => {
                if current_layers.contains(&l) {
                    current_layers.retain(|x| *x != l);
                }
                current_layers.push(l);
            }
            Action::Layer(l, false) => {
                current_layers.retain(|x| *x != l);
            }
        }
    }
}
