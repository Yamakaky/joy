use anyhow::Result;
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use joycon::hidapi::{HidApi, DeviceInfo, HidDevice};
use joycon::joycon_sys::{HID_IDS, NINTENDO_VENDOR_ID};
use std::{
    collections::{HashMap, HashSet},
    mem::swap,
    sync::mpsc::Sender,
    thread::sleep,
    time::{Duration, Instant},
};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Table, Tabs, Widget},
    Terminal,
};

enum Event {
    CEvent(CEvent),
    NewDevice(String, HidDevice),
    DisconnectedDevice(String),
    Tick,
}

pub fn run() -> Result<()> {
    enable_raw_mode()?;

    let (tx, rx) = std::sync::mpsc::channel();
    start_input_loop(tx.clone());
    start_hidapi_loop(tx)?;

    let mut devices = HashMap::<String, HidDevice>::new();
    let mut good = true;
    let mut device_selected = 0;

    let mut menu_selected = ListState::default();
    menu_selected.select(Some(0));

    let stdout = std::io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    'main: loop {
        terminal.draw(|frame| {
            let size = frame.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(2)])
                .split(size);

            let mut macs = devices.keys().collect::<Vec<_>>();
            macs.sort();
            let device_selector = Tabs::new(
                macs.into_iter()
                    .map(|mac| Spans::from(mac.clone()))
                    .collect(),
            )
            .select(device_selected)
            .block(Block::default().title("Devices").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow))
            .divider(Span::raw("|"));

            frame.render_widget(device_selector, chunks[0]);

            let display_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
                .split(chunks[1]);
            let (menu, display) = render_device(&menu_selected);
            frame.render_stateful_widget(menu, display_chunks[0], &mut menu_selected);
            frame.render_widget(menu, display_chunks[1]);
        })?;

        match rx.recv()? {
            Event::CEvent(CEvent::Key(k)) => match k.code {
                KeyCode::Char('q') => {
                    disable_raw_mode()?;
                    terminal.clear()?;
                    terminal.show_cursor()?;
                    break 'main;
                }
                KeyCode::Tab => {
                    device_selected = (device_selected + 1).min(devices.len() - 1);
                }
                KeyCode::BackTab => {
                    device_selected = device_selected.saturating_sub(1);
                }
                _ => {}
            },
            Event::CEvent(CEvent::Mouse(_)) => {}
            Event::CEvent(CEvent::Resize(_, _)) => {}
            Event::NewDevice(serial, device) => {
                devices.insert(serial, device);
            }
            Event::DisconnectedDevice(serial) => {
                devices.remove(&serial);
            }
            Event::Tick => {
                good = false;
                if let Some(dev) = devices.values().next() {
                    good = dev.read_timeout(&mut [0; 500], 1).is_ok();
                }
            }
        };
    }
    Ok(())
}

fn render_device(menu_state: &ListState) -> (List, Table) {
    let menu = List::new()
    Table::new(vec![ListItem::new(content)]);
    (menu, values)
}

fn start_input_loop(tx: Sender<Event>) {
    let tick_rate = Duration::from_millis(200);
    std::thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                let event = event::read().expect("can read events");
                tx.send(Event::CEvent(event)).expect("can send events");
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });
}

fn start_hidapi_loop(tx: Sender<Event>) -> Result<()> {
    let mut api = HidApi::new()?;
    std::thread::spawn(move || {
        let mut known = HashSet::new();
        let mut new_known = HashSet::new();
        loop {
            api.refresh_devices().unwrap();
            let devices: HashMap<String, &DeviceInfo> = api
                .device_list()
                .filter(|x| {
                    x.vendor_id() == NINTENDO_VENDOR_ID && HID_IDS.contains(&x.product_id())
                })
                .map(|i| (i.serial_number().unwrap().to_string(), i))
                .collect();
            for (serial, info) in devices {
                if !known.contains(&serial) {
                    match info.open_device(&api) {
                        Ok(device) => tx.send(Event::NewDevice(serial.clone(), device)).unwrap(),
                        Err(e) => eprintln!("hidapi error: {:?}", e),
                    }
                }
                new_known.insert(serial);
            }
            for serial in known.difference(&new_known) {
                tx.send(Event::DisconnectedDevice(serial.clone())).unwrap();
            }
            swap(&mut known, &mut new_known);
            new_known.clear();
            sleep(Duration::from_millis(200));
        }
    });
    Ok(())
}
