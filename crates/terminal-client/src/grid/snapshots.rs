use std::{cmp::max, collections::HashSet};

use sanedit_messages::redraw::{
    snapshots::{SnapshotPoint, Snapshots},
    Cell, ThemeField,
};
use sanedit_utils::bitset::Bitset256;

use crate::{
    grid::{
        drawable::{DrawCursor, Drawable, Subgrid},
        Rect, Split,
    },
    ui::UIContext,
};

#[derive(Debug)]
pub(crate) struct CustomSnapshots {
    pub(crate) snapshots: Snapshots,
    pub(crate) scroll: usize,
}

impl CustomSnapshots {
    pub fn new(snapshots: Snapshots) -> CustomSnapshots {
        CustomSnapshots {
            snapshots,
            scroll: 0,
        }
    }

    pub fn split_off(&self, win: &mut Rect) -> Rect {
        const MIN: usize = 30;
        // Each level is indented by 2, and root starts at indent 2, +1 for possible directory marker
        // let max_item_width = self
        //     .items
        //     .items
        //     .iter()
        //     .map(|item| (item.level + 1) * 2 + item.name.chars().count() + 1)
        //     .max()
        //     .unwrap_or(0)
        //     + 1;
        let max_screen = max(MIN, win.width / 3);
        // let width = max_item_width.clamp(MIN, max_screen);
        win.split_off(Split::left_size(max_screen))
    }

    pub fn update_scroll_position(&mut self, rect: &Rect) {
        let height = rect.height;
        let sel = self.snapshots.selected;
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

impl Drawable for CustomSnapshots {
    fn draw(&self, ctx: &UIContext, mut cells: Subgrid) {
        let markers = ctx.style(ThemeField::FiletreeMarkers);
        let default = ctx.style(ThemeField::FiletreeDefault);
        let entry = ctx.style(ThemeField::FiletreeDir);

        let sforward = "│";
        let sfork = "├";
        let shoriz = "─";
        let sfork_continue = "┴";
        let spoint_continue = "┼";
        let sroot = "┴";
        let sleaf = "┬";

        cells.clear_all(default);
        let sep = Cell::new_char('│', markers);
        let sub = cells.draw_separator_right(sep);
        let mut content_area = cells.subgrid(&sub);
        let rendered = render_snapshots(&self.snapshots);

        log::info!("--------------");
        for (i, snap) in self.snapshots.points.iter().enumerate() {
            log::info!("{i}: {snap:?}");
        }

        for (row, line) in rendered.iter().enumerate() {
            content_area.put_string(row, 0, &line.graph, default);
        }
    }

    fn cursor(&self, ctx: &UIContext) -> DrawCursor {
        if self.snapshots.in_focus {
            DrawCursor::Hide
        } else {
            DrawCursor::Ignore
        }
    }
}

struct RenderedLine {
    graph: String,
    title: String,
}

fn render_snapshots(snapshots: &Snapshots) -> Vec<RenderedLine> {
    let mut result = vec![];
    let mut used_lanes = Bitset256::new();
    dfs(&snapshots.points, 0, 0, &mut used_lanes, &mut result);
    result
}

fn format_lanes_before(max: u8, used_lanes: &Bitset256) -> String {
    log::info!("MAX: {max:?}, {used_lanes:?}");
    let mut result = String::new();
    for i in 0..max {
        if used_lanes.contains(i) {
            result.push_str("│ ");
        } else {
            result.push_str("  ");
        }
    }

    result
}

fn dfs(
    snapshots: &[SnapshotPoint],
    node: usize,
    lane: usize,
    used_lanes: &Bitset256,
    result: &mut Vec<RenderedLine>,
) {
    if lane > u8::MAX as usize {
        return;
    }

    let mut lanes = used_lanes.clone();
    lanes.insert(lane as u8);

    let lanes_before = format_lanes_before(lane as u8, used_lanes);
    let point = &snapshots[node];

    // Leaf
    if point.next.is_empty() {
        let line = RenderedLine {
            graph: format!("{}{}", lanes_before, "┬ "),
            title: point.title.clone(),
        };
        result.push(line);
        return;
    }

    let last = point.next.len() - 1;
    dfs(snapshots, point.next[last], lane, &lanes, result);

    for (i, n) in point.next[..last].iter().rev().enumerate() {
        dfs(snapshots, *n, lane + 1 + i, &lanes, result);
    }

    let nmid_lanes = point.next.len().saturating_sub(2);
    let fork = point.next.len() > 1;
    let first = node == 0;
    let (mylane, next_lanes) = match (lane, fork, first) {
        (0, false, true) => ("┴ ", String::new()),
        (0, true, true) => ("┴─", format!("{}{}", "┴─".repeat(nmid_lanes), "┘ ")),
        (_, true, _) => ("├─", format!("{}{}", "┴─".repeat(nmid_lanes), "┘ ")),
        (_, false, _) => ("┼ ", String::new()),
    };

    let line = RenderedLine {
        graph: format!("{}{}{}", lanes_before, mylane, next_lanes),
        title: point.title.clone(),
    };
    result.push(line);
}
