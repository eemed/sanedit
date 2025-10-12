use std::cmp::max;

use sanedit_messages::redraw::{
    items::{Item, ItemKind, ItemLocation, Items},
    Cell, Size, Style, ThemeField,
};

use crate::ui::UIContext;

use super::{
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

            let opts = FormatItemOptions {
                line: row,
                name_style: name,
                fill_style: fill,
                mark_style: markers,
                match_style: markers,
                width,
            };
            Self::format_ft_item(&mut grid, item, &opts);
        }
    }

    fn format_ft_item(grid: &mut Subgrid<'_, '_>, item: &Item, opts: &FormatItemOptions) {
        let FormatItemOptions {
            line,
            name_style,
            fill_style,
            mark_style,
            width,
            ..
        } = opts;
        let mut x = 0;
        for _ in 0..item.level {
            x += grid.put_string(*line, x, "  ", *fill_style);
        }

        match item.kind {
            ItemKind::Group { expanded } => {
                if expanded {
                    x += grid.put_string(*line, x, "-", *mark_style);
                } else {
                    x += grid.put_string(*line, x, "+", *mark_style);
                }
            }
            ItemKind::Item => {
                x += grid.put_string(*line, x, "#", *mark_style);
            }
        }

        grid.put_string(*line, x, " ", *fill_style);
        x += 1;

        let is_group = matches!(item.kind, ItemKind::Group { .. });
        let suffix = if is_group { 1 } else { 0 };
        let available = width.saturating_sub(x + suffix);
        let nlen = item.name.chars().count();
        let start = nlen.saturating_sub(available);
        let px = x;
        for ch in item
            .name
            .chars()
            .skip(start)
            .map(|ch| if ch.is_control() { ' ' } else { ch })
        {
            if x >= *width {
                break;
            }

            grid.replace(*line, x, Cell::new_char(ch, *name_style));
            x += 1;
        }
        if start != 0 && nlen > 2 && px + 1 < *width {
            grid.at(*line, px).text = ".".into();
            grid.at(*line, px + 1).text = ".".into();
        }

        if is_group {
            x += grid.put_string(*line, x, "/", *name_style);
        }

        while x < *width {
            grid.replace(*line, x, Cell::with_style(*fill_style));
            x += 1;
        }
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

        let mut x = 0;
        let loading = if self.items.is_loading { " (..)" } else { "" };
        let title_text = format!(" Locations ({}){}", self.items.title, loading);

        x += grid.put_string(0, x, &title_text, title);
        while x < width {
            grid.replace(0, x, Cell::with_style(title));
            x += 1;
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
            let opts = FormatItemOptions {
                line: row,
                name_style: style,
                fill_style: fil,
                mark_style: mark,
                match_style: mat,
                width,
            };
            Self::format_loc_item(&mut grid, item, &opts);
        }
    }

    fn format_loc_item(grid: &mut Subgrid<'_, '_>, item: &Item, opts: &FormatItemOptions) {
        let FormatItemOptions {
            line,
            name_style,
            fill_style,
            mark_style,
            match_style,
            width,
        } = opts;
        let mut x = 0;
        for _ in 0..item.level {
            x += grid.put_string(*line, x, "  ", *fill_style)
        }

        match item.kind {
            ItemKind::Group { expanded } => {
                if expanded {
                    x += grid.put_string(*line, x, "- ", *mark_style);
                } else {
                    x += grid.put_string(*line, x, "+ ", *mark_style);
                }

                let available = width.saturating_sub(x);
                let nlen = item.name.chars().count();
                let start = nlen.saturating_sub(available);

                let px = x;
                for ch in item
                    .name
                    .chars()
                    .skip(start)
                    .map(|ch| if ch.is_control() { ' ' } else { ch })
                {
                    if x >= *width {
                        break;
                    }

                    grid.replace(*line, x, Cell::new_char(ch, *name_style));
                    x += 1;
                }
                if start != 0 && nlen > 2  && px + 1 < *width {
                    grid.at(*line, px).text = ".".into();
                    grid.at(*line, px + 1).text = ".".into();
                }

                while x < *width {
                    grid.replace(*line, x, Cell::with_style(*fill_style));
                    x += 1;
                }
            }
            ItemKind::Item => {
                let prefix = item
                    .location
                    .as_ref()
                    .map(|loc| match loc {
                        ItemLocation::Line(n) => format!("{n}"),
                        ItemLocation::ByteOffset(n) => format!("{n:#x}"),
                    })
                    .unwrap_or("?".into());
                let line_start = format!("{prefix}: ");
                x += grid.put_string(*line, x, &line_start, *mark_style);
                let px = x;
                x += grid.put_string(*line, x, &item.name, *name_style);

                // Highlight matches
                for hl in &item.highlights {
                    let mut pos = 0;
                    let mut i = px;
                    while i < x {
                        let cell = grid.at(*line, i);
                        if hl.contains(&pos) {
                            cell.style = *match_style;
                        }
                        pos += cell.text.len();
                        i += 1;
                    }
                }

                while x < *width {
                    grid.replace(*line, x, Cell::with_style(*fill_style));
                    x += 1;
                }
            }
        }
    }
}

struct FormatItemOptions {
    line: usize,
    name_style: Style,
    fill_style: Style,
    mark_style: Style,
    match_style: Style,
    width: usize,
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
