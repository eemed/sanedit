use std::{
    cmp::{max, min},
    mem,
    ops::{Deref, DerefMut},
};

use sanedit_messages::redraw::{
    Cell, Component, Cursor, CursorShape, Diffable, IntoCells, Point, Prompt, Redraw, Severity,
    Size, Source, StatusMessage, Statusline, Style, ThemeField, Window,
};

use crate::ui::UIContext;

pub(crate) struct Grid {
    size: Size,
    window: Rectangle<Window>,
    statusline: Rectangle<Statusline>,
    // gutter: Option<Rectangle<()>>,
    prompt: Option<Rectangle<CustomPrompt>>,
    msg: Option<Rectangle<StatusMessage>>,

    drawn: Vec<Vec<Cell>>,
    cursor: Cursor,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Grid {
        let size = Size { width, height };
        let mut window = Rect {
            x: 0,
            y: 0,
            width,
            height,
        };
        let statusline = window.split_off(Split::top_size(1));

        Grid {
            size,
            window: Rectangle::new(Window::default(), window),
            statusline: Rectangle::new(Statusline::default(), statusline),
            prompt: None,
            msg: None,

            drawn: vec![vec![Cell::default(); width]; height],
            cursor: Cursor::default(),
        }
    }

    pub fn on_send_input(&mut self) {
        self.msg = None;
    }

    pub fn handle_redraw(&mut self, ctx: &UIContext, msg: Redraw) {
        use Component::*;
        use Redraw::*;

        match msg {
            Window(comp) => match comp {
                Open(win) => self.window.inner = win,
                Update(diff) => self.window.inner.update(diff),
                Close => {}
            },
            Statusline(comp) => match comp {
                Open(status) => self.statusline.inner = status,
                Update(diff) => self.statusline.inner.update(diff),
                Close => {}
            },
            Prompt(comp) => match comp {
                Open(prompt) => {
                    let olay = Rect::top_center(self.size.width, self.size.height);
                    let required = prompt.max_completions + 4;
                    let style = match prompt.source {
                        Source::Search => PromptStyle::Oneline,
                        Source::Prompt => {
                            if olay.height < required {
                                PromptStyle::Oneline
                            } else {
                                PromptStyle::Overlay
                            }
                        }
                    };
                    let rect = match style {
                        PromptStyle::Oneline => {
                            let mut rect = self.statusline.rect.clone();
                            rect.height = min(required, self.size.height);
                            Rectangle::new(CustomPrompt { prompt, style }, rect)
                        }
                        PromptStyle::Overlay => {
                            let mut olay = olay;
                            olay.height = min(olay.height, required);
                            Rectangle::new(CustomPrompt { prompt, style }, olay)
                        }
                    };
                    self.prompt = Some(rect);
                }
                Update(diff) => {
                    if let Some(ref mut prompt) = self.prompt {
                        prompt.inner.prompt.update(diff);
                    }
                }
                Close => self.prompt = None,
            },
            StatusMessage(msg) => {
                let rect = Rect {
                    x: 0,
                    y: 0,
                    width: self.size.width,
                    height: 1,
                };
                self.msg = Some(Rectangle::new(msg, rect));
            }
            _ => {} // Completion(comp) => match comp {
                    //     Open(compl) => self.completion = Some(compl),
                    //     Update(diff) => {
                    //         if let Some(ref mut compl) = self.completion {
                    //             compl.update(diff);
                    //         }
                    //     }
                    //     Close => self.completion = None,
                    // },
                    // LineNumbers(numbers) => {
                    //     let gutter = Gutter::new(numbers);
                    //     ctx.gutter_size = gutter.width();
                    //     self.gutter = gutter.into()
                    // }
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        if let Some(ref mut prompt) = self.prompt {
            let rect = &mut prompt.rect;
            rect.width = min(rect.width, width);
            rect.height = min(rect.height, height);
            rect.x = 0;
            rect.y = 0;
            prompt.inner.style = PromptStyle::Oneline;
        }

        if let Some(ref mut msg) = self.msg {
            let rect = &mut msg.rect;
            rect.width = min(rect.width, width);
            rect.height = min(rect.height, height);
            rect.x = 0;
            rect.y = 0;
        }

        let rect = &mut self.statusline.rect;
        rect.width = min(rect.width, width);

        let rect = &mut self.window.rect;
        rect.width = min(rect.width, width);
        rect.height = min(rect.width, height);
    }

    pub fn window_rect(&self) -> Rect {
        self.window.rect.clone()
    }

    pub fn clear(&mut self) {
        for row in self.drawn.iter_mut() {
            for cell in row.iter_mut() {
                *cell = Cell::default();
            }
        }
    }

    fn draw_drawable<D: Drawable>(
        drawable: &Rectangle<D>,
        ctx: &UIContext,
        cursor: &mut Cursor,
        cells: &mut Vec<Vec<Cell>>,
    ) {
        let rect = drawable.rect.clone();
        if let Some(cur) = drawable.cursor(ctx) {
            *cursor = cur;
            cursor.point = cursor.point + rect.position();
        }

        let top_left = rect.position();
        let mut grid = rect.grid();
        let mut g: Vec<&mut [CCell]> = grid.iter_mut().map(|v| v.as_mut_slice()).collect();
        drawable.draw(ctx, &mut g);

        for (line, row) in grid.into_iter().enumerate() {
            for (col, cell) in row.into_iter().enumerate() {
                if cell.is_transparent {
                    continue;
                }
                let x = top_left.x + col;
                let y = top_left.y + line;
                cells[y][x] = cell.cell;
            }
        }
    }

    pub fn draw(&mut self, ctx: &UIContext) -> (&Vec<Vec<Cell>>, Cursor) {
        self.clear();

        Self::draw_drawable(&self.window, ctx, &mut self.cursor, &mut self.drawn);
        Self::draw_drawable(&self.statusline, ctx, &mut self.cursor, &mut self.drawn);

        if let Some(ref prompt) = self.prompt {
            Self::draw_drawable(prompt, ctx, &mut self.cursor, &mut self.drawn);
        }

        (&self.drawn, self.cursor)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Border {
    Box,
    Margin,
}

impl Border {
    pub fn top_left(&self) -> &str {
        match self {
            Border::Box => "┌",
            Border::Margin => " ",
        }
    }

    pub fn top_right(&self) -> &str {
        match self {
            Border::Box => "┐",
            Border::Margin => " ",
        }
    }

    pub fn bottom_right(&self) -> &str {
        match self {
            Border::Box => "┘",
            Border::Margin => " ",
        }
    }

    pub fn bottom_left(&self) -> &str {
        match self {
            Border::Box => "└",
            Border::Margin => " ",
        }
    }

    pub fn bottom(&self) -> &str {
        match self {
            Border::Box => "─",
            Border::Margin => " ",
        }
    }

    pub fn top(&self) -> &str {
        match self {
            Border::Box => "─",
            Border::Margin => " ",
        }
    }

    pub fn left(&self) -> &str {
        match self {
            Border::Box => "│",
            Border::Margin => " ",
        }
    }

    pub fn right(&self) -> &str {
        match self {
            Border::Box => "│",
            Border::Margin => " ",
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum PromptStyle {
    /// Simple one line prompt with options on another lines
    Oneline,
    /// An overlay window
    Overlay,
}

#[derive(Debug)]
struct CustomPrompt {
    style: PromptStyle,
    prompt: Prompt,
}

pub(crate) struct Rectangle<T>
where
    T: Drawable,
{
    inner: T,
    rect: Rect,
}

impl<T: Drawable> Rectangle<T> {
    pub fn new(t: T, rect: Rect) -> Rectangle<T> {
        Rectangle { inner: t, rect }
    }
}

impl<T: Drawable> Drawable for Rectangle<T> {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        self.inner.draw(ctx, cells);
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Cursor> {
        self.inner.cursor(ctx)
    }
}

fn draw_border_with_style<'a, 'b, F: Fn(usize, usize) -> Style>(
    border: Border,
    get_style: F,
    mut cells: &'a mut [&'b mut [CCell]],
) -> &'a mut [&'b mut [CCell]] {
    let size = size(cells);

    if size.width <= 2 && size.height <= 2 {
        return cells;
    }

    // Top and bottom
    for i in 1..size.width {
        cells[0][i] = Cell {
            text: border.top().into(),
            style: get_style(0, i),
        }
        .into();
        cells[size.height - 1][i] = Cell {
            text: border.bottom().into(),
            style: get_style(size.height - 1, i),
        }
        .into();
    }

    // Sides
    for i in 1..size.height {
        cells[i][0] = Cell {
            text: border.left().into(),
            style: get_style(i, 0),
        }
        .into();
        cells[i][size.width - 1] = Cell {
            text: border.right().into(),
            style: get_style(i, size.width),
        }
        .into();
    }

    // corners
    cells[0][0] = Cell {
        text: border.top_left().into(),
        style: get_style(0, 0),
    }
    .into();

    cells[size.height - 1][0] = Cell {
        text: border.bottom_left().into(),
        style: get_style(size.height - 1, 0),
    }
    .into();

    cells[0][size.width - 1] = Cell {
        text: border.top_right().into(),
        style: get_style(0, size.width - 1),
    }
    .into();

    cells[size.height - 1][size.width - 1] = Cell {
        text: border.bottom_right().into(),
        style: get_style(size.height - 1, size.width - 1),
    }
    .into();

    cells = &mut cells[1..size.height - 1];
    for i in 0..cells.len() {
        let line = mem::replace(&mut cells[i], &mut []);
        let width = line.len();
        cells[i] = &mut line[1..width - 1];
    }
    cells
}

/// Draw border and return inner cells to draw to
fn draw_border<'a, 'b>(
    border: Border,
    style: Style,
    cells: &'a mut [&'b mut [CCell]],
) -> &'a mut [&'b mut [CCell]] {
    draw_border_with_style(border, |_, _| style, cells)
}

pub(crate) enum SplitPoint {
    Percentage(usize),
    Size(usize),
}

impl SplitPoint {
    pub fn get(&self, size: usize) -> usize {
        match self {
            SplitPoint::Percentage(p) => (size * p) / 100,
            SplitPoint::Size(s) => min(*s, size),
        }
    }
}

pub(crate) enum Split {
    Top(SplitPoint),
    Bottom(SplitPoint),
    Left(SplitPoint),
    Right(SplitPoint),
}

impl Split {
    pub fn top_size(size: usize) -> Split {
        Split::Top(SplitPoint::Size(size))
    }

    pub fn bottom_size(size: usize) -> Split {
        Split::Bottom(SplitPoint::Size(size))
    }

    pub fn left_size(size: usize) -> Split {
        Split::Left(SplitPoint::Size(size))
    }

    pub fn right_size(size: usize) -> Split {
        Split::Right(SplitPoint::Size(size))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Rect {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl Rect {
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    pub fn prompt_overlay(width: usize, height: usize, maxheight: usize) {}

    pub fn top_center(width: usize, height: usize) -> Rect {
        let width = width / 2;
        let height = height / 2;
        let x = width / 2;
        let y = height / 4;

        Rect {
            x,
            y,
            width,
            height,
        }
    }

    pub fn centered(width: usize, height: usize) -> Rect {
        let width = width / 2;
        let height = height / 2;
        let x = width / 2;
        let y = height / 2;

        Rect {
            x,
            y,
            width,
            height,
        }
    }

    pub fn position(&self) -> Point {
        Point {
            x: self.x,
            y: self.y,
        }
    }

    pub fn size(&self) -> Size {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    pub fn grid(&self) -> Vec<Vec<CCell>> {
        vec![vec![CCell::transparent(); self.width]; self.height]
    }

    pub fn split_off(&mut self, split: Split) -> Rect {
        match split {
            Split::Top(split) => {
                let amount = split.get(self.height);
                self.y += amount;
                self.height -= amount;

                Rect {
                    x: self.x,
                    y: self.y - amount,
                    width: self.width,
                    height: amount,
                }
            }
            Split::Bottom(split) => {
                let amount = split.get(self.height);
                self.height -= amount;

                Rect {
                    x: self.x,
                    y: self.y + self.height,
                    width: self.width,
                    height: amount,
                }
            }
            Split::Left(split) => {
                let amount = split.get(self.width);
                self.x += amount;
                self.width -= amount;

                Rect {
                    x: self.x - amount,
                    y: self.y,
                    width: amount,
                    height: self.height,
                }
            }
            Split::Right(split) => {
                let amount = split.get(self.width);
                self.width -= amount;

                Rect {
                    x: self.x + self.width,
                    y: self.y,
                    width: amount,
                    height: self.height,
                }
            }
        }
    }
}

pub(crate) trait Drawable {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]);
    fn cursor(&self, ctx: &UIContext) -> Option<Cursor>;
}

impl Drawable for Window {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        let width = min(
            cells.get(0).map(|c| c.len()).unwrap_or(0),
            self.cells.get(0).map(|c| c.len()).unwrap_or(0),
        );
        let height = min(cells.len(), self.cells.len());

        for x in 0..width {
            for y in 0..height {
                cells[y][x] = self.cells[y][x].clone().into();
            }
        }
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Cursor> {
        Some(self.cursor)
    }
}

impl Drawable for Statusline {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        let style = ctx.style(ThemeField::Statusline);
        let width = cells.get(0).map(|c| c.len()).unwrap_or(0);
        for (i, cell) in into_cells_with_theme_pad_with(&self.line, &style, width)
            .into_iter()
            .enumerate()
        {
            cells[0][i] = cell;
        }
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Cursor> {
        None
    }
}

impl Drawable for CustomPrompt {
    fn draw(&self, ctx: &UIContext, mut cells: &mut [&mut [CCell]]) {
        let wsize = size(cells);
        let default_style = ctx.theme.get(ThemeField::PromptDefault);
        let input_style = ctx.theme.get(ThemeField::PromptUserInput);

        match self.style {
            PromptStyle::Oneline => {
                let mut message = into_cells_with_style(
                    &self.prompt.message,
                    ctx.theme.get(ThemeField::PromptTitle),
                );
                let colon = into_cells_with_style(": ", ctx.theme.get(ThemeField::PromptTitle));
                let input = into_cells_with_style(&self.prompt.input, input_style);
                message.extend(colon);
                message.extend(input);
                pad_line(&mut message, default_style, wsize.width);
                put_line(message, 0, cells);
            }
            PromptStyle::Overlay => {
                if wsize.height > 2 {
                    let title = into_cells_with_style(
                        &self.prompt.message,
                        ctx.theme.get(ThemeField::PromptTitle),
                    );
                    let title = center_pad(title, default_style, wsize.width);
                    put_line(title, 0, cells);

                    let mut message =
                        into_cells_with_style(" > ", ctx.theme.get(ThemeField::PromptMessage));
                    let input = into_cells_with_style(&self.prompt.input, input_style);
                    message.extend(input);
                    pad_line(&mut message, default_style, wsize.width);
                    put_line(message, 1, cells);
                }

                cells = &mut cells[2..];

                let pcompl = ctx.theme.get(ThemeField::PromptCompletion);
                set_style(cells, pcompl);
                cells = draw_border(Border::Margin, pcompl, cells);
                let wsize = size(cells);
                let max_opts = wsize.height;

                self.prompt
                    .options
                    .iter()
                    .take(max_opts)
                    .enumerate()
                    .for_each(|(i, opt)| {
                        let field = if Some(i) == self.prompt.selected {
                            ThemeField::PromptCompletionSelected
                        } else {
                            ThemeField::PromptCompletion
                        };
                        let style = ctx.style(field);
                        put_line(into_cells_with_style_pad(opt, style, wsize.width), i, cells);
                    });
            }
        }
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Cursor> {
        match self.style {
            PromptStyle::Oneline => {
                let cursor_col = {
                    let input_cells_before_cursor =
                        self.prompt.input[..self.prompt.cursor].into_cells().len();
                    let msg = self.prompt.message.chars().count();
                    let extra = 2; // ": "
                    msg + extra + input_cells_before_cursor
                };
                let style = ctx.theme.get(ThemeField::Default);
                Some(Cursor {
                    bg: style.fg,
                    fg: style.bg,
                    point: Point {
                        x: cursor_col,
                        y: 0,
                    },
                    shape: CursorShape::Line(true),
                })
            }
            PromptStyle::Overlay => {
                let cursor_col = {
                    let input_cells_before_cursor =
                        self.prompt.input[..self.prompt.cursor].into_cells().len();
                    let extra = 3; // " > "
                    extra + input_cells_before_cursor
                };
                let style = ctx.theme.get(ThemeField::Default);
                Some(Cursor {
                    bg: style.fg,
                    fg: style.bg,
                    point: Point {
                        x: cursor_col,
                        y: 1,
                    },
                    shape: CursorShape::Line(true),
                })
            }
        }
    }
}

impl Drawable for StatusMessage {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        let field = match self.severity {
            Severity::Info => ThemeField::Info,
            Severity::Warn => ThemeField::Warn,
            Severity::Error => ThemeField::Error,
        };
        let style = ctx.style(field);
        let width = cells.get(0).map(|c| c.len()).unwrap_or(0);
        for (i, cell) in into_cells_with_theme_pad_with(&self.message, &style, width)
            .into_iter()
            .enumerate()
        {
            cells[0][i] = cell;
        }
    }

    fn cursor(&self, ctx: &UIContext) -> Option<Cursor> {
        None
    }
}

fn into_cells(string: &str) -> Vec<CCell> {
    string.chars().map(|ch| CCell::from(ch)).collect()
}

fn into_cells_with_style(string: &str, style: Style) -> Vec<CCell> {
    let mut cells = into_cells(string);
    cells.iter_mut().for_each(|cell| cell.style = style);
    cells
}

fn into_cells_with_style_pad(string: &str, style: Style, width: usize) -> Vec<CCell> {
    let mut cells = into_cells_with_style(string, style);
    pad_line(&mut cells, style, width);
    cells
}

fn into_cells_with_theme_pad_with(string: &str, style: &Style, width: usize) -> Vec<CCell> {
    let mut cells = into_cells_with_theme_with(string, style);
    pad_line(&mut cells, style.clone(), width);
    cells
}

fn into_cells_with_theme_with(string: &str, style: &Style) -> Vec<CCell> {
    let mut cells = into_cells(string);
    cells.iter_mut().for_each(|cell| cell.style = style.clone());
    cells
}

fn pad_line(cells: &mut Vec<CCell>, style: Style, width: usize) {
    while cells.len() < width {
        cells.push(CCell::with_style(style.clone()));
    }

    while cells.len() > width {
        cells.pop();
    }
}

fn size(cells: &mut [&mut [CCell]]) -> Size {
    let height = cells.len();
    let width = cells.get(0).map(|line| line.len()).unwrap_or(0);

    Size { width, height }
}

fn put_line(line: Vec<CCell>, pos: usize, target: &mut [&mut [CCell]]) {
    for (i, cell) in line.into_iter().enumerate() {
        target[pos][i] = cell;
    }
}

fn set_style(target: &mut [&mut [CCell]], style: Style) {
    for line in target.iter_mut() {
        for cell in line.iter_mut() {
            cell.style = style.clone();
        }
    }
}

fn center_pad(message: Vec<CCell>, pad_style: Style, width: usize) -> Vec<CCell> {
    let pad = (width.saturating_sub(message.len())) / 2;
    let mut result = into_cells_with_style(&" ".repeat(pad), pad_style);
    result.extend(message);
    pad_line(&mut result, pad_style, width);
    result
}

#[derive(Debug, Clone)]
pub struct CCell {
    is_transparent: bool,
    cell: Cell,
}

impl CCell {
    pub fn transparent() -> CCell {
        CCell {
            is_transparent: true,
            cell: Cell::default(),
        }
    }

    pub fn from(ch: char) -> CCell {
        CCell {
            is_transparent: false,
            cell: Cell::from(ch),
        }
    }

    pub fn with_style(style: Style) -> CCell {
        CCell {
            is_transparent: false,
            cell: Cell::with_style(style),
        }
    }
}

impl Default for CCell {
    fn default() -> Self {
        CCell {
            is_transparent: false,
            cell: Cell::default(),
        }
    }
}

impl Deref for CCell {
    type Target = Cell;

    fn deref(&self) -> &Self::Target {
        &self.cell
    }
}

impl DerefMut for CCell {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cell
    }
}

impl From<Cell> for CCell {
    fn from(value: Cell) -> Self {
        CCell {
            is_transparent: false,
            cell: value,
        }
    }
}
