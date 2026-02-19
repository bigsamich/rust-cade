use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, KeyEvent, KeyEventKind};

pub enum Event {
    Key(KeyEvent),
    Tick,
}

pub struct EventHandler {
    rx: mpsc::Receiver<Event>,
}

impl EventHandler {
    pub fn new(tick_rate_ms: u64) -> Self {
        let (tx, rx) = mpsc::channel();
        let tick_rate = Duration::from_millis(tick_rate_ms);

        thread::spawn(move || loop {
            if event::poll(tick_rate).unwrap_or(false) {
                if let Ok(crossterm::event::Event::Key(key)) = event::read() {
                    if key.kind == KeyEventKind::Press {
                        if tx.send(Event::Key(key)).is_err() {
                            return;
                        }
                    }
                }
            } else if tx.send(Event::Tick).is_err() {
                return;
            }
        });

        Self { rx }
    }

    pub fn next(&self) -> io::Result<Event> {
        self.rx
            .recv()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
}
