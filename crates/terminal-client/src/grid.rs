mod border;
mod completion;
mod drawable;
mod items;
mod popup;
mod prompt;
mod rect;
mod snapshots;
mod window;

use std::sync::Arc;

use completion::CustomCompletion;
use drawable::Subgrid;
use items::Kind;
use popup::popup_rect;
use sanedit_messages::{
    redraw::{
        completion::CompletionUpdate,
        items::ItemsUpdate,
        prompt::PromptUpdate,
        snapshots::SnapshotsUpdate,
        statusline::Statusline,
        window::{Window, WindowUpdate},
        Cell, Cursor, Popup, PopupComponent, Redraw, Size, StatusMessage, Theme,
    },
    Message,
};

use crate::{grid::snapshots::CustomSnapshots, ui::UIContext};

pub(crate) use self::rect::{Rect, Split};
use self::{
    drawable::{DrawCursor, Drawable},
    items::CustomItems,
    prompt::CustomPrompt,
};

/// An item placed on a rectangle
pub(crate) struct Placed<T> {
    pub(crate) item: T,
    pub(crate) rect: Rect,
}

impl<T: Default> Default for Placed<T> {
    fn default() -> Self {
        Placed {
            item: T::default(),
            rect: Rect::default(),
        }
    }
}

impl<T> From<T> for Placed<T> {
    fn from(value: T) -> Self {
        Placed {
            item: value,
            rect: Rect::default(),
        }
    }
}

pub(crate) struct Grid {
    size: Size,
    window: Placed<Window>,
    statusline: Placed<Statusline>,
    prompt: Option<Placed<CustomPrompt>>,
    msg: Option<Placed<StatusMessage>>,
    completion: Option<Placed<CustomCompletion>>,
    filetree: Option<Placed<CustomItems>>,
    locations: Option<Placed<CustomItems>>,
    snapshots: Option<Placed<CustomSnapshots>>,
    popup: Option<Placed<Popup>>,

    drawn: Vec<Vec<Cell>>,
    cursor: Option<Cursor>,
    pub theme: Arc<Theme>,
    pub client_in_focus: bool,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Grid {
        let mut me = Grid {
            size: Size { width, height },
            window: Placed::default(),
            statusline: Placed::default(),
            prompt: None,
            msg: None,
            completion: None,
            filetree: None,
            locations: None,
            snapshots: None,
            popup: None,

            drawn: vec![vec![Cell::default(); width]; height],
            cursor: None,
            theme: Arc::new(Theme::default()),
            client_in_focus: true,
        };
        me.refresh();
        me
    }

    pub fn on_send_input(&mut self, _msg: &Message) {
        self.msg = None;
    }

    pub fn on_focus_change(&mut self, focus: bool) {
        self.client_in_focus = focus;
    }

    pub fn handle_redraw(&mut self, msg: Redraw) -> RedrawResult {
        use Redraw::*;

        match msg {
            Window(update) => match update {
                WindowUpdate::Full(win) => self.window.item = win,
                WindowUpdate::Cursor(cursor) => self.window.item.cursor = cursor,
            },
            Statusline(statusline) => self.statusline.item = statusline,
            Prompt(update) => match update {
                PromptUpdate::Full(prompt) => match self.prompt {
                    Some(ref mut custom_prompt) => {
                        let is_new = custom_prompt.item.prompt.message != prompt.message;
                        custom_prompt.item.prompt = prompt;
                        if is_new {
                            self.refresh_overlays();
                        }
                    }
                    None => {
                        self.prompt = Some(CustomPrompt::new(prompt).into());
                        self.refresh_overlays();
                    }
                },
                PromptUpdate::Selection(pos) => {
                    if let Some(prompt) = &mut self.prompt {
                        prompt.item.prompt.selected = pos;
                    }
                }
                PromptUpdate::Close => self.prompt = None,
            },
            StatusMessage(msg) => {
                self.msg = Some(Placed {
                    item: msg,
                    rect: self.statusline.rect,
                });
            }
            Completion(update) => {
                match update {
                    CompletionUpdate::Full(completion) => match self.completion {
                        Some(ref mut compl) => {
                            compl.item.update(completion);
                        }
                        None => {
                            self.completion = Some(CustomCompletion::new(completion).into());
                        }
                    },
                    CompletionUpdate::Selection(pos) => {
                        if let Some(compl) = &mut self.completion {
                            compl.item.completion.selected = pos;
                        }
                    }
                    CompletionUpdate::Close => self.completion = None,
                }

                self.refresh_overlays();
            }
            Filetree(update) => match update {
                ItemsUpdate::Full(items) => {
                    match self.filetree {
                        Some(ref mut ft) => {
                            ft.item.items = items;
                            self.refresh();
                        }
                        None => {
                            self.filetree = Some(CustomItems::new(items, Kind::Filetree).into());
                            self.refresh();
                        }
                    }
                    return RedrawResult::Resized;
                }
                ItemsUpdate::Selection(pos) => {
                    if let Some(ft) = &mut self.filetree {
                        ft.item.items.selected = pos.unwrap_or(0);
                        self.refresh();
                    }
                }
                ItemsUpdate::Close => {
                    self.filetree = None;
                    self.refresh();
                    return RedrawResult::Resized;
                }
            },
            Locations(update) => match update {
                ItemsUpdate::Full(items) => {
                    match self.locations {
                        Some(ref mut locs) => {
                            locs.item.items = items;
                            self.refresh();
                        }
                        None => {
                            self.locations = Some(CustomItems::new(items, Kind::Locations).into());
                            self.refresh();
                        }
                    }
                    return RedrawResult::Resized;
                }
                ItemsUpdate::Selection(pos) => {
                    if let Some(loc) = &mut self.locations {
                        loc.item.items.selected = pos.unwrap_or(0);
                        self.refresh();
                    }
                }
                ItemsUpdate::Close => {
                    self.locations = None;
                    self.refresh();
                    return RedrawResult::Resized;
                }
            },
            Popup(popup) => match popup {
                PopupComponent::Open(popup) => {
                    self.popup = Some(popup.into());
                    self.refresh_overlays();
                }
                PopupComponent::Close => {
                    self.popup = None;
                }
            },
            Snapshots(update) => match update {
                SnapshotsUpdate::Full(nsnaps) => {
                    match self.snapshots {
                        Some(ref mut snaps) => {
                            snaps.item.snapshots = nsnaps;
                            self.refresh();
                        }
                        None => {
                            self.snapshots = Some(CustomSnapshots::new(nsnaps).into());
                            self.refresh();
                        }
                    }
                    return RedrawResult::Resized;
                }
                SnapshotsUpdate::Selection(pos) => {
                    if let Some(snaps) = &mut self.snapshots {
                        snaps.item.snapshots.selected = pos.unwrap_or(0);
                        self.refresh();
                    }
                }
                SnapshotsUpdate::Close => {
                    self.snapshots = None;
                    self.refresh();
                    return RedrawResult::Resized;
                }
            },
        }

        RedrawResult::Ok
    }

    fn screen(&self) -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: self.size.width,
            height: self.size.height,
        }
    }

    fn refresh_overlays(&mut self) {
        let screen = self.screen();
        let win = self.window();

        if let Some(compl) = &mut self.completion {
            let new = compl.item.rect(win);
            let rect = &mut compl.rect;
            // Update only if bigger or old old does not fit
            if !rect.includes(&new) || !win.includes(rect) {
                *rect = new
            }
        }

        if let Some(prompt) = &mut self.prompt {
            prompt.rect = prompt.item.rect(screen);
        }

        if let Some(popup) = &mut self.popup {
            popup.rect = popup_rect(screen, win, &popup.item);
        }
    }

    /// Calculate locations for all
    pub fn refresh(&mut self) {
        let mut window = self.screen();
        self.statusline.rect = window.split_off(Split::top_size(1));

        // Message same as statusline
        if let Some(msg) = &mut self.msg {
            msg.rect = self.statusline.rect;
        }

        if let Some(snaps) = &mut self.snapshots {
            snaps.rect = snaps.item.split_off(&mut window);
            snaps.item.update_scroll_position(&snaps.rect);
        }

        if let Some(ft) = &mut self.filetree {
            ft.rect = ft.item.split_off(&mut window);
            ft.item.update_scroll_position(&ft.rect);
        }

        if let Some(loc) = &mut self.locations {
            loc.rect = loc.item.split_off(&mut window);
            loc.item.update_scroll_position(&loc.rect);
        }

        self.window.rect = window;
        self.refresh_overlays();
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.size.width = width;
        self.size.height = height;
        self.drawn = vec![vec![Cell::default(); width]; height];
        self.refresh();
    }

    pub fn window(&self) -> Rect {
        self.window.rect
    }

    pub fn filetree(&mut self) -> Option<&mut Placed<CustomItems>> {
        self.filetree.as_mut()
    }

    pub fn locations(&mut self) -> Option<&mut Placed<CustomItems>> {
        self.locations.as_mut()
    }

    pub fn clear(&mut self) {
        for row in self.drawn.iter_mut() {
            for cell in row.iter_mut() {
                *cell = Cell::default();
            }
        }

        self.cursor = None;
    }

    fn draw_drawable<D: Drawable>(
        drawable: &D,
        rect: &Rect,
        theme: &Arc<Theme>,
        client_in_focus: bool,
        cursor: &mut Option<Cursor>,
        cells: &mut Vec<Vec<Cell>>,
    ) {
        let ctx = UIContext {
            theme: theme.clone(),
            rect: *rect,
            client_in_focus,
            cursor_position: cursor.as_ref().map(|c| c.point).unwrap_or_default(),
        };

        match drawable.cursor(&ctx) {
            DrawCursor::Hide => *cursor = None,
            DrawCursor::Show(mut cur) => {
                cur.point = cur.point + rect.position();
                *cursor = Some(cur);
            }
            DrawCursor::Ignore => {}
        }

        let subgrid = Subgrid { cells, rect };
        drawable.draw(&ctx, subgrid);
    }

    pub fn draw(&mut self) -> (&Vec<Vec<Cell>>, Option<Cursor>) {
        self.clear();

        let t = &self.theme;
        Self::draw_drawable(
            &self.window.item,
            &self.window.rect,
            t,
            self.client_in_focus,
            &mut self.cursor,
            &mut self.drawn,
        );
        Self::draw_drawable(
            &self.statusline.item,
            &self.statusline.rect,
            t,
            self.client_in_focus,
            &mut self.cursor,
            &mut self.drawn,
        );

        if let Some(loc) = &self.locations {
            Self::draw_drawable(
                &loc.item,
                &loc.rect,
                t,
                self.client_in_focus,
                &mut self.cursor,
                &mut self.drawn,
            );
        }

        if let Some(snaps) = &self.snapshots {
            Self::draw_drawable(
                &snaps.item,
                &snaps.rect,
                t,
                self.client_in_focus,
                &mut self.cursor,
                &mut self.drawn,
            );
        }

        if let Some(ft) = &self.filetree {
            Self::draw_drawable(
                &ft.item,
                &ft.rect,
                t,
                self.client_in_focus,
                &mut self.cursor,
                &mut self.drawn,
            );
        }

        if let Some(msg) = &self.msg {
            Self::draw_drawable(
                &msg.item,
                &msg.rect,
                t,
                self.client_in_focus,
                &mut self.cursor,
                &mut self.drawn,
            );
        }

        if let Some(compl) = &self.completion {
            Self::draw_drawable(
                &compl.item,
                &compl.rect,
                t,
                self.client_in_focus,
                &mut self.cursor,
                &mut self.drawn,
            );
        }

        if let Some(popup) = &self.popup {
            Self::draw_drawable(
                &popup.item,
                &popup.rect,
                t,
                self.client_in_focus,
                &mut self.cursor,
                &mut self.drawn,
            );
        }

        if let Some(prompt) = &self.prompt {
            Self::draw_drawable(
                &prompt.item,
                &prompt.rect,
                t,
                self.client_in_focus,
                &mut self.cursor,
                &mut self.drawn,
            );
        }

        (&self.drawn, self.cursor)
    }
}

pub(crate) enum RedrawResult {
    Ok,
    Resized,
}
