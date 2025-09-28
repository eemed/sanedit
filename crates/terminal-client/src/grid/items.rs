use std::cmp::max;

use sanedit_messages::redraw::{
    items::{Item, ItemKind, ItemLocation, Items},
    Cell, Size, Style, ThemeField,
};

use crate::ui::UIContext;

use super::{
    cell_format::{into_cells_with_style, pad_line},
    drawable::{DrawCursor, Drawable, Subgrid},
    Rect, Split,
};

#[derive(Debug)]
pub(crate) enum Kind {
    Filetree,
    Locations,
}

#[derive(Debug)]
pub(crate) struct CustomItems {
    pub(crate) items: Items,
    pub(crate) scroll: usize,
    pub(crate) kind: Kind,
}

impl CustomItems {
    pub fn new(items: Items, kind: Kind) -> CustomItems {
        CustomItems {
            items,
            scroll: 0,
            kind,
        }
    }

    pub fn split_off(&self, win: &mut Rect) -> Rect {
        match self.kind {
            Kind::Filetree => {
                const MIN: usize = 30;
                // Each level is indented by 2, and root starts at indent 2, +1 for possible directory marker
                let max_item_width = self
                    .items
                    .items
                    .iter()
                    .map(|item| (item.level + 1) * 2 + item.name.chars().count() + 1)
                    .max()
                    .unwrap_or(0)
                    + 1;
                let max_screen = max(MIN, win.width / 3);
                let width = max_item_width.clamp(MIN, max_screen);
                win.split_off(Split::left_size(width))
            }
            Kind::Locations => {
                // +1 = title
                let items = self.items.items.len() + 1;
                let height = items.clamp(3, 15);
                win.split_off(Split::bottom_size(height))
            }
        }
    }

    pub fn update_scroll_position(&mut self, rect: &Rect) {
        let area_reserved = if matches!(self.kind, Kind::Locations) {
            1
        } else {
            0
        };
        let height = rect.height.saturating_sub(area_reserved);
        let sel = self.items.selected;
        let at_least = sel.saturating_sub(height.saturating_sub(1));
        self.scroll = max(self.scroll, at_least);

        if self.scroll > sel {
            self.scroll = sel;
        }

        if self.scroll + height < sel {
            self.scroll = sel - (height / 2);
        }
    }
}

impl CustomItems {
    fn draw_filetree(&self, ctx: &UIContext, mut grid: Subgrid) {
        let fill = ctx.style(ThemeField::FiletreeDefault);
        let file = ctx.style(ThemeField::FiletreeFile);
        let dir = ctx.style(ThemeField::FiletreeDir);
        let markers = ctx.style(ThemeField::FiletreeMarkers);
        let selfill = ctx.style(ThemeField::FiletreeSelected);
        let sel = ctx.style(ThemeField::FiletreeSelectedFile);
        let dsel = ctx.style(ThemeField::FiletreeSelectedDir);
        let msel = ctx.style(ThemeField::FiletreeSelectedMarkers);

        grid.clear_all(fill);

        let sep = Cell::new_char('│', markers);
        let inside = grid.draw_separator_right(sep);
        let mut grid = grid.subgrid(&inside);

        let width = grid.width();
        for (row, item) in self.items.items.iter().skip(self.scroll).enumerate() {
            if row >= grid.height() {
                break;
            }

            let is_selected = self.scroll + row == self.items.selected;
            let (name, fill, markers) = {
                if is_selected {
                    let name = match item.kind {
                        ItemKind::Group { .. } => dsel,
                        ItemKind::Item => sel,
                    };
                    (name, selfill, msel)
                } else {
                    let name = match item.kind {
                        ItemKind::Group { .. } => dir,
                        ItemKind::Item => file,
                    };
                    (name, fill, markers)
                }
            };

            let titem = Self::format_ft_item(item, name, fill, markers, width);

            for (i, cell) in titem.into_iter().enumerate() {
                grid.replace(row, i, cell);
            }
        }
    }

    fn format_ft_item(
        item: &Item,
        nstyle: Style,
        fill_style: Style,
        mark_style: Style,
        width: usize,
    ) -> Vec<Cell> {
        let mut result = vec![];
        result.extend(into_cells_with_style(&"  ".repeat(item.level), fill_style));

        match item.kind {
            ItemKind::Group { expanded } => {
                if expanded {
                    result.extend(into_cells_with_style("-", mark_style));
                } else {
                    result.extend(into_cells_with_style("+", mark_style));
                }
            }
            ItemKind::Item => {
                result.extend(into_cells_with_style("#", mark_style));
            }
        }
        result.extend(into_cells_with_style(" ", fill_style));

        let is_group = matches!(item.kind, ItemKind::Group { .. });
        let prefix = result.len();
        let suffix = if is_group { 1 } else { 0 };
        let available = width.saturating_sub(prefix + suffix);
        let name = into_cells_with_style(&item.name, nstyle);
        let start = name.len().saturating_sub(available);
        let mut cells: Vec<Cell> = name.into_iter().skip(start).collect();
        if start != 0 && cells.len() > 2 {
            cells[0].text = ".".into();
            cells[1].text = ".".into();
        }
        result.extend(cells);

        if is_group {
            result.extend(into_cells_with_style("/", nstyle));
        }

        pad_line(&mut result, fill_style, width);
        result
    }

    fn draw_locations(&self, ctx: &UIContext, mut grid: Subgrid) {
        let fill = ctx.style(ThemeField::LocationsDefault);
        let entry = ctx.style(ThemeField::LocationsEntry);
        let group = ctx.style(ThemeField::LocationsGroup);
        let markers = ctx.style(ThemeField::LocationsMarkers);
        let smarkers = ctx.style(ThemeField::LocationsSelectedMarkers);
        let title = ctx.style(ThemeField::LocationsTitle);
        let sel = ctx.style(ThemeField::LocationsSelectedEntry);
        let gsel = ctx.style(ThemeField::LocationsSelectedGroup);
        let lmat = ctx.style(ThemeField::LocationsMatch);
        let smat = ctx.style(ThemeField::LocationsSelectedMatch);

        grid.clear_all(fill);

        let Size { width, .. } = grid.size();
        if grid.height() == 0 {
            return;
        }

        let loading = if self.items.is_loading { " (..)" } else { "" };
        let title_text = format!(" Locations ({}){}", self.items.title, loading);

        let mut line = into_cells_with_style(&title_text, title);

        for _ in line.len()..width {
            let mut ccell = Cell::from(' ');
            ccell.style = title;
            line.push(ccell);
        }

        line.truncate(width);

        for (i, c) in line.into_iter().enumerate() {
            grid.replace(0, i, c);
        }

        let mut rect = *grid.rect();
        rect.y += 1;
        rect.height = rect.height.saturating_sub(1);
        let mut grid = grid.subgrid(&rect);

        for (row, item) in self.items.items.iter().skip(self.scroll).enumerate() {
            if row >= grid.height() {
                break;
            }

            let width = grid.width();
            let is_selected = self.scroll + row == self.items.selected;
            let style = {
                if is_selected {
                    match item.kind {
                        ItemKind::Group { .. } => gsel,
                        ItemKind::Item => sel,
                    }
                } else {
                    match item.kind {
                        ItemKind::Group { .. } => group,
                        ItemKind::Item => entry,
                    }
                }
            };
            let mat = if is_selected { smat } else { lmat };
            let fil = if is_selected { sel } else { fill };
            let mark = if is_selected { smarkers } else { markers };
            let titem = Self::format_loc_item(item, style, mark, mat, fil, width);

            for (i, cell) in titem.into_iter().enumerate() {
                grid.replace(row, i, cell);
            }
        }
    }

    fn format_loc_item(
        item: &Item,
        name_style: Style,
        extra_style: Style,
        match_style: Style,
        fill_style: Style,
        width: usize,
    ) -> Vec<Cell> {
        let mut result = vec![];
        result.extend(into_cells_with_style(&"  ".repeat(item.level), fill_style));

        match item.kind {
            ItemKind::Group { expanded } => {
                if expanded {
                    result.extend(into_cells_with_style("- ", extra_style));
                } else {
                    result.extend(into_cells_with_style("+ ", extra_style));
                }

                let prefix = result.len();
                let available = width.saturating_sub(prefix);
                let name = into_cells_with_style(&item.name, name_style);
                let start = name.len().saturating_sub(available);
                let mut cells: Vec<Cell> = name.into_iter().skip(start).collect();
                if start != 0 && cells.len() > 2 {
                    cells[0].text = ".".into();
                    cells[1].text = ".".into();
                }
                result.extend(cells);
                pad_line(&mut result, fill_style, width);
            }
            ItemKind::Item => {
                let line = item
                    .location
                    .as_ref()
                    .map(|loc| match loc {
                        ItemLocation::Line(n) => format!("{n}"),
                        ItemLocation::ByteOffset(n) => format!("{n:#x}"),
                    })
                    .unwrap_or("?".into());
                result.extend(into_cells_with_style(&format!("{line}: "), extra_style));
                let mut name = into_cells_with_style(&item.name, name_style);

                // Highlight matches
                for hl in &item.highlights {
                    let mut pos = 0;
                    for cell in &mut name {
                        if hl.contains(&pos) {
                            cell.style = match_style;
                        }
                        pos += cell.text.len();
                    }
                }

                result.extend(name);
                pad_line(&mut result, fill_style, width);
            }
        }

        result
    }
}

impl Drawable for CustomItems {
    fn draw(&self, ctx: &UIContext, grid: Subgrid) {
        match self.kind {
            Kind::Filetree => self.draw_filetree(ctx, grid),
            Kind::Locations => self.draw_locations(ctx, grid),
        }
    }

    fn cursor(&self, _ctx: &UIContext) -> DrawCursor {
        if self.items.in_focus {
            DrawCursor::Hide
        } else {
            DrawCursor::Ignore
        }
    }
}
