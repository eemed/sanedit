use std::cmp::max;

use sanedit_messages::redraw::{
    items::{Difference, Item, ItemKind, Items},
    Diffable, Size, Style, ThemeField,
};

use crate::ui::UIContext;

use super::{
    ccell::{clear_all, into_cells_with_style, pad_line, size, CCell},
    drawable::{DrawCursor, Drawable},
    Rect,
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

    pub fn update(&mut self, diff: Difference, rect: Rect) {
        self.items.update(diff);

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
    }
}

impl CustomItems {
    fn draw_filetree(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        let fill = ctx.style(ThemeField::FiletreeDefault);
        let file = ctx.style(ThemeField::FiletreeFile);
        let dir = ctx.style(ThemeField::FiletreeDir);
        let markers = ctx.style(ThemeField::FiletreeMarkers);
        let sel = ctx.style(ThemeField::FiletreeSelectedFile);
        let dsel = ctx.style(ThemeField::FiletreeSelectedDir);

        clear_all(cells, fill);

        let Size { width, .. } = size(cells);
        let last = width.saturating_sub(1);

        for l in cells.iter_mut() {
            let mut ccell = CCell::from('│');
            ccell.style = markers;
            l[last] = ccell;
        }

        for l in cells.iter_mut() {
            let line = std::mem::take(l);
            *l = &mut line[..last];
        }

        let Size { width, .. } = size(cells);
        for (row, item) in self.items.items.iter().skip(self.scroll).enumerate() {
            if row >= cells.len() {
                break;
            }

            let style = {
                if self.scroll + row == self.items.selected {
                    match item.kind {
                        ItemKind::Group { .. } => dsel,
                        ItemKind::Item => sel,
                    }
                } else {
                    match item.kind {
                        ItemKind::Group { .. } => dir,
                        ItemKind::Item => file,
                    }
                }
            };

            let mut titem = Self::format_ft_item(item, style, markers);
            pad_line(&mut titem, fill, width);

            for (i, cell) in titem.into_iter().enumerate() {
                cells[row][i] = cell;
            }
        }
    }

    fn format_ft_item(item: &Item, name: Style, extra: Style) -> Vec<CCell> {
        let mut result = vec![];
        result.extend(into_cells_with_style(&"  ".repeat(item.level), extra));

        match item.kind {
            ItemKind::Group { expanded } => {
                if expanded {
                    result.extend(into_cells_with_style("- ", extra));
                } else {
                    result.extend(into_cells_with_style("+ ", extra));
                }
            }
            ItemKind::Item => {
                result.extend(into_cells_with_style("# ", extra));
            }
        }

        result.extend(into_cells_with_style(&item.name, name));

        if matches!(item.kind, ItemKind::Group { .. }) {
            result.extend(into_cells_with_style("/", name));
        }

        result
    }

    fn draw_locations(&self, ctx: &UIContext, mut cells: &mut [&mut [CCell]]) {
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

        clear_all(cells, fill);

        let Size { width, .. } = size(cells);
        if !cells.is_empty() {
            let mut line = into_cells_with_style(" Locations", title);

            for _ in line.len()..cells[0].len() {
                let mut ccell = CCell::from(' ');
                ccell.style = title;
                line.push(ccell);
            }

            line.truncate(width);

            for (i, c) in line.into_iter().enumerate() {
                cells[0][i] = c;
            }

            cells = &mut cells[1..];
        }

        for (row, item) in self.items.items.iter().skip(self.scroll).enumerate() {
            if row >= cells.len() {
                break;
            }

            let width = cells.first().map(|c| c.len()).unwrap_or(0);
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
            let mut titem = Self::format_loc_item(item, style, mark, mat, fil);
            pad_line(&mut titem, fil, width);

            for (i, cell) in titem.into_iter().enumerate() {
                cells[row][i] = cell;
            }
        }
    }

    fn format_loc_item(
        item: &Item,
        name: Style,
        extra: Style,
        mat: Style,
        fill: Style,
    ) -> Vec<CCell> {
        let mut result = vec![];
        result.extend(into_cells_with_style(&"  ".repeat(item.level), fill));

        match item.kind {
            ItemKind::Group { expanded } => {
                if expanded {
                    result.extend(into_cells_with_style("- ", extra));
                } else {
                    result.extend(into_cells_with_style("+ ", extra));
                }
            }
            ItemKind::Item => {
                let line = item.line.map(|l| l.to_string()).unwrap_or("?".into());
                result.extend(into_cells_with_style(&format!("{line}: "), extra));
            }
        }

        let mut name = into_cells_with_style(&item.name, name);

        // Highlight matches
        for hl in &item.highlights {
            let mut pos = 0;
            // dont count padding
            for cell in &mut name {
                if hl.contains(&pos) {
                    cell.style = mat;
                }
                pos += cell.cell.text.len();
            }
        }

        result.extend(name);
        result
    }
}

impl Drawable for CustomItems {
    fn draw(&self, ctx: &UIContext, cells: &mut [&mut [CCell]]) {
        match self.kind {
            Kind::Filetree => self.draw_filetree(ctx, cells),
            Kind::Locations => self.draw_locations(ctx, cells),
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
