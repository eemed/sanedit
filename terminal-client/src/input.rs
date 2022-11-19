use crossterm::event::{KeyCode, KeyModifiers};
use sanedit_messages::{Key, KeyEvent, KeyMods};

pub(crate) fn run_loop(stop: &AtomicBool) {
}

// const POLL_DURATION: Duration = Duration::from_millis(100);
// const RE_RESIZE_POLL_DURATION: Duration = Duration::from_millis(100);

// pub(crate) fn input_loop(mut handle: EditorHandle) -> Result<()> {
//     loop {
//         if poll(POLL_DURATION)? {
//             let event = read()?;
//             process_input_event(event, &mut handle)?;
//         }

//         if !IS_RUNNING.load(Ordering::Acquire) {
//             break;
//         }
//     }

//     Ok(())
// }

// fn process_input_event(event: crossterm::event::Event, handle: &mut EditorHandle) -> Result<()> {
//     use crossterm::event::Event::*;

//     match event {
//         Key(key_event) => {
//             let key = convert_key_event(key_event);
//             if !handle.send_key_press(key) {
//                 return Err(anyhow!("Failed to send key press"));
//             }
//         }
//         Resize(mut width, mut height) => {
//             while poll(RE_RESIZE_POLL_DURATION)? {
//                 let e = read()?;
//                 match e {
//                     Resize(w, h) => {
//                         width = w;
//                         height = h;
//                     }
//                     _ => {
//                         resize(width as usize, height as usize, handle)?;
//                         process_input_event(e, handle)?;
//                         return Ok(());
//                     }
//                 }
//             }

//             resize(width as usize, height as usize, handle)?;
//         }
//         Mouse(mouse_event) => {
//             let result = match mouse_event.kind {
//                 MouseEventKind::ScrollDown => handle.mouse_scroll_down(),
//                 MouseEventKind::ScrollUp => handle.mouse_scroll_up(),
//                 _ => true,
//             };

//             if !result {
//                 return Err(anyhow!("Failed to mouse event"));
//             }
//         }
//     }

//     Ok(())
// }

pub(crate) fn convert_key_event(key: crossterm::event::KeyEvent) -> sanedit_messages::KeyEvent {
    let plain_key = match key.code {
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Enter => Key::Enter,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Tab => Key::Tab,
        KeyCode::BackTab => Key::BackTab,
        KeyCode::Delete => Key::Delete,
        KeyCode::Insert => Key::Insert,
        KeyCode::F(n) => Key::F(n),
        KeyCode::Char(ch) => Key::Char(ch),
        KeyCode::Esc => Key::Esc,
        KeyCode::Null => Key::Unknown,
    };

    let mut mods = KeyMods::empty();

    if key.modifiers.contains(KeyModifiers::ALT) {
        mods |= KeyMods::ALT;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        mods |= KeyMods::CONTROL;
    }

    KeyEvent::new(plain_key, mods)
}
