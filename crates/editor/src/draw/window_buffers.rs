use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc,
};

use sanedit_messages::{
    redraw::{window::Window, Component, Redraw},
    ClientMessage,
};
use sanedit_server::{FromEditor, FromEditorSharedMessage};

#[derive(Debug)]
pub(crate) struct WindowBuffers {
    zero: Arc<FromEditor>,
    zero_notif: (Sender<()>, Receiver<()>),
    one: Arc<FromEditor>,
    one_notif: (Sender<()>, Receiver<()>),
    active: usize,
}

impl Default for WindowBuffers {
    fn default() -> Self {
        let (tx0, rx0) = channel();
        let (tx1, rx1) = channel();

        let _ = tx0.send(());
        let _ = tx1.send(());

        WindowBuffers {
            zero: Arc::new(FromEditor::Message(ClientMessage::Redraw(Redraw::Window(
                Component::Open(Window::default()),
            )))),
            zero_notif: (tx0, rx0),
            one: Arc::new(FromEditor::Message(ClientMessage::Redraw(Redraw::Window(
                Component::Open(Window::default()),
            )))),
            one_notif: (tx1, rx1),
            active: 0,
        }
    }
}

impl WindowBuffers {
    pub fn get(&self) -> FromEditorSharedMessage {
        let (item, (send, _)) = if self.active == 0 {
            (&self.zero, &self.zero_notif)
        } else {
            (&self.one, &self.one_notif)
        };

        FromEditorSharedMessage::Window {
            notify: send.clone(),
            message: item.clone(),
        }
    }

    pub fn get_window(&self) -> &Window {
        let (item, (_, _)) = if self.active == 0 {
            (&self.zero, &self.zero_notif)
        } else {
            (&self.one, &self.one_notif)
        };

        if let FromEditor::Message(ClientMessage::Redraw(Redraw::Window(Component::Open(win)))) =
            item.as_ref()
        {
            return win;
        }

        unreachable!()
    }

    pub fn next_mut(&mut self) -> &mut Window {
        let (item, (_, notif)) = if self.active == 0 {
            self.active = 1;
            (&mut self.one, &mut self.one_notif)
        } else {
            self.active = 0;
            (&mut self.zero, &mut self.zero_notif)
        };

        let _ = notif.recv();
        let item = Arc::make_mut(item);
        if let FromEditor::Message(ClientMessage::Redraw(Redraw::Window(Component::Open(win)))) =
            item
        {
            return win;
        }

        unreachable!()
    }
}
