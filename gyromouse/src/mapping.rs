use std::collections::HashMap;
use std::time::Instant;

pub struct KeyEntry {
    pub on_down: Option<Action>,
    pub on_up: Option<Action>,
    pub on_hold: Option<Action>,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Action {
    KeyPress(char, Option<bool>),
    Layer(u32, Option<bool>),
}

pub struct Joystick {
    layers: HashMap<u32, HashMap<JoyKey, KeyEntry>>,
    current_layer: Vec<u32>,
    current_keys: HashMap<JoyKey, Instant>,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
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

impl Joystick {
    pub fn new() -> Joystick {
        Joystick {
            layers: HashMap::new(),
            current_layer: vec![0],
            current_keys: HashMap::new(),
        }
    }

    pub fn add_layer(&mut self, id: u32, layer: HashMap<JoyKey, KeyEntry>) {
        self.layers.insert(id, layer);
    }

    pub fn key_down(&mut self, key: JoyKey) {
        self.current_keys.insert(key, Instant::now());
        for layer in self.current_layer.clone().into_iter().rev() {
            if let Some(entry) = self.layers[&layer].get(&key) {
                if let Some(action) = entry.on_down {
                    if entry.on_hold.is_none() {
                        self.exec(action);
                        break;
                    }
                }
            }
        }
    }

    pub fn key_up(&mut self, key: JoyKey) {
        for layer in self.current_layer.clone().into_iter().rev() {
            if let Some(entry) = self.layers[&layer].get(&key) {
                if let Some(hold_action) = entry.on_hold {
                    if self
                        .current_keys
                        .get(&key)
                        .map(|x| x.elapsed().as_millis() < 100)
                        .unwrap_or(false)
                    {
                        if let Some(down_action) = entry.on_down {
                            self.exec(down_action);
                            break;
                        }
                        if let Some(up_action) = entry.on_up {
                            self.exec(up_action);
                            break;
                        }
                    } else {
                        self.exec(hold_action);
                        break;
                    }
                } else if let Some(up_action) = entry.on_up {
                    self.exec(up_action);
                    break;
                }
            }
        }
        self.current_keys.remove(&key);
    }

    fn exec(&mut self, action: Action) {
        match action {
            Action::KeyPress(c, None) => println!("click {}", c),
            Action::KeyPress(c, Some(true)) => println!("down {}", c),
            Action::KeyPress(c, Some(false)) => println!("up {}", c),
            Action::Layer(ref l, None) => {
                if self.current_layer.contains(l) {
                    self.current_layer.retain(|x| x != l);
                } else {
                    self.current_layer.push(*l);
                }
            }
            Action::Layer(ref l, Some(true)) => {
                if self.current_layer.contains(&l) {
                    self.current_layer.retain(|x| x != l);
                }
                self.current_layer.push(*l);
            }
            Action::Layer(ref l, Some(false)) => {
                self.current_layer.retain(|x| x != l);
            }
        }
    }
}
