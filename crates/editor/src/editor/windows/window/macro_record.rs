use sanedit_messages::key::KeyEvent;

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

    pub fn name(&self) -> Option<&str>  {
        self.name.as_ref().map(String::as_str)
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
