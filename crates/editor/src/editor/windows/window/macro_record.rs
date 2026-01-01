use std::collections::VecDeque;

use sanedit_messages::key::KeyEvent;

#[derive(Debug, Default, Clone)]
pub struct MacroReplay {
    keys: VecDeque<KeyEvent>,
    is_replaying: bool,
}

impl MacroReplay {
    pub fn is_replaying(&self) -> bool {
        self.is_replaying
    }

    pub fn replay(&mut self, keys: VecDeque<KeyEvent>) {
        self.keys = keys;
        self.is_replaying = true;
    }

    pub fn stop_replaying(&mut self) {
        self.keys.clear();
        self.is_replaying = false;
    }

    pub fn pop(&mut self) -> Option<KeyEvent> {
        self.keys.pop_front()
    }
}

#[derive(Debug, Default)]
pub struct MacroRecord {
    events: Vec<KeyEvent>,
    is_recording: bool,
    name: Option<String>,
}

impl MacroRecord {
    pub fn record_named(&mut self, name: &str) {
        self.name = Some(name.into());
        self.record();
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn record(&mut self) {
        self.events.clear();
        self.is_recording = true;
    }

    pub fn events(&self) -> &[KeyEvent] {
        &self.events
    }

    pub fn stop_recording(&mut self) {
        self.is_recording = false;
    }

    pub fn push_event(&mut self, event: KeyEvent) {
        if self.is_recording {
            self.events.push(event);
        }
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording
    }
}
