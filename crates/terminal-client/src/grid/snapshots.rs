use std::{cmp::max, collections::HashSet};

use sanedit_messages::redraw::{snapshots::Snapshots, Cell, ThemeField};

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
        const MIN: usize = 15;

        let rendered = render_snapshots(&self.snapshots);
        let max_width = rendered
            .iter()
            .map(|line| line.graph.chars().count() + 2 + line.title.chars().count())
            .max()
            .unwrap_or(0);
        let max_screen = max(MIN, win.width / 3);
        let width = max_width.clamp(MIN, max_screen);
        win.split_off(Split::left_size(width))
    }

    pub fn update_scroll_position(&mut self, rect: &Rect) {
        let height = rect.height;
        let sel = self.snapshots.selected;

        let mut visible_n = 0;
        let mut rows = 0;
        for point in &self.snapshots.points {
            if point.next.len() > 1 {
                rows += 2;
            } else {
                rows += 1;
            }

            if sel == visible_n {
                break;
            }
            visible_n += 1;
        }

        let mut min_scroll = 0;
        let mut iter = self.snapshots.points.iter();
        while rows > height {
            let point = iter.next().unwrap();

            if point.next.len() > 1 {
                rows -= 2;
            } else {
                rows -= 1;
            }

            min_scroll += 1;
        }

        let mut max_scroll = min_scroll;
        while rows != 0 {
            let point = iter.next().unwrap();

            if point.next.len() > 1 {
                rows -= 2;
            } else {
                rows -= 1;
            }

            if rows != 0 {
                max_scroll += 1;
            }
        }

        self.scroll = self.scroll.clamp(min_scroll, max_scroll)
    }
}

impl Drawable for CustomSnapshots {
    fn draw(&self, ctx: &UIContext, mut cells: Subgrid) {
        let markers = ctx.style(ThemeField::SnapshotsMarkers);
        let default = ctx.style(ThemeField::SnapshotsDefault);
        let graph = ctx.style(ThemeField::SnapshotsGraph);
        let graph_point = ctx.style(ThemeField::SnapshotsGraphPoint);
        let label = ctx.style(ThemeField::SnapshotsLabel);
        let sel = ctx.style(ThemeField::SnapshotsSelected);
        let selgraph = ctx.style(ThemeField::SnapshotsSelectedGraph);
        let selgraph_point = ctx.style(ThemeField::SnapshotsSelectedGraphPoint);
        let sellabel = ctx.style(ThemeField::SnapshotsSelectedLabel);

        cells.clear_all(default);
        let sep = Cell::new_char('│', markers);
        let sub = cells.draw_separator_right(sep);
        let mut content_area = cells.subgrid(&sub);
        let rendered = render_snapshots(&self.snapshots);
        let mut skipped = 0;

        for (row, line) in rendered
            .iter()
            .filter(|line| {
                if skipped < self.scroll {
                    if line.graph.contains("*") {
                        skipped += 1;
                    }
                    return false;
                }

                true
            })
            .enumerate()
        {
            if row >= content_area.height() {
                break;
            }

            if line.selected {
                content_area.style_line(row, sel);
            }
            let (sgraph, sgraph_point, slabel) = if line.selected {
                (selgraph, selgraph_point, sellabel)
            } else {
                (graph, graph_point, label)
            };
            let mut x = 0;

            for (i, split) in line.graph.split("*").enumerate() {
                if i & 1 == 1 && x < content_area.width() {
                    x += content_area.put_string(row, x, "*", sgraph_point);
                }

                if x < content_area.width() {
                    x += content_area.put_string(row, x, split, sgraph);
                }
            }

            let left = content_area.width().saturating_sub(x);
            let size = line.title.chars().count();
            let at = if left > size {
                content_area.width() - size
            } else {
                content_area.width().saturating_sub(left)
            };
            content_area.put_string(row, at, &line.title, slabel);
        }
    }

    fn cursor(&self, _ctx: &UIContext) -> DrawCursor {
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
    selected: bool,
}

#[derive(Clone)]
struct PointData {
    lane: usize,
    used_lanes: HashSet<usize>,
}

// This is terrible, but drawing graphs is terrible so i dont care
fn render_snapshots(snapshots: &Snapshots) -> Vec<RenderedLine> {
    let mut stack = vec![PointData {
        lane: 0,
        used_lanes: HashSet::new(),
    }];
    let mut result = Vec::with_capacity(snapshots.points.len());

    // Start iterating from bottom up
    for (i, point) in snapshots.points.iter().rev().enumerate() {
        let selected = snapshots.points.len() - 1 - i == snapshots.selected;
        let last_saved = snapshots.points.len() - 1 - i == snapshots.last_saved;
        let title = if last_saved {
            format!("(s) {}", point.title)
        } else {
            point.title.clone()
        };
        let Some(PointData { lane, used_lanes }) = stack.pop() else {
            continue;
        };

        let mut lanes = used_lanes.clone();
        lanes.insert(lane);

        let lanes_before = format_lanes_before(lane, &used_lanes);
        let is_leaf = point.next.is_empty();

        if is_leaf {
            let symbol = "* ";
            let line = RenderedLine {
                graph: format!("{lanes_before}{symbol}"),
                title,
                selected,
            };
            result.push(line);
            continue;
        }

        let last = point.next.len() - 1;
        stack.push(PointData {
            lane: lane,
            used_lanes: lanes.clone(),
        });

        let mut next_lanes = String::new();

        for (i, _) in point.next[..last].iter().rev().enumerate() {
            let next_lane = lane + i + 1;
            stack.push(PointData {
                lane: next_lane,
                used_lanes: lanes.clone(),
            });
            lanes.insert(next_lane);

            if i + 1 == last {
                next_lanes.push_str("┘ ");
            } else {
                next_lanes.push_str("┴─");
            }
        }

        let fork = point.next.len() > 1;
        let first = point.id == 0;
        let mylane = match (lane, fork, first) {
            (0, false, true) => "* ",
            (0, true, true) => "┴─",
            (_, false, _) => "* ",
            (_, true, _) => {
                let line = RenderedLine {
                    graph: format!("{}{}{}", lanes_before, "* ", String::new()),
                    title,
                    selected,
                };
                result.push(line);
                let line = RenderedLine {
                    graph: format!("{}{}{}", lanes_before, "├─", next_lanes),
                    title: String::new(),
                    selected: false,
                };
                result.push(line);
                continue;
            }
        };

        let line = RenderedLine {
            graph: format!("{}{}{}", lanes_before, mylane, next_lanes),
            title,
            selected,
        };
        result.push(line);
    }

    // Reverse order again as we iterated in reverse
    result.reverse();
    result
}

fn format_lanes_before(max: usize, used_lanes: &HashSet<usize>) -> String {
    let mut result = String::new();
    for i in 0..max {
        if used_lanes.contains(&i) {
            result.push_str("│ ");
        } else {
            result.push_str("  ");
        }
    }

    result
}
