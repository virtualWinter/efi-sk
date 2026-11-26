use super::{Entry, Groups, failure_message};
use alloc::format;
use core::fmt::Write;
use uefi::proto::console::text::{Color, Output};
use uefi::{Status, system};

fn line(out: &mut Output, x: usize, y: usize, width: usize, value: &str, active: bool) {
    let colors = if active {
        (Color::White, Color::Blue)
    } else {
        (Color::LightGray, Color::Black)
    };
    let _ = out.set_color(colors.0, colors.1);
    let _ = out.set_cursor_position(x, y);
    // One printable ASCII character per cell; firmware glyph support varies.
    for ch in value.chars().take(width) {
        let _ = out.write_char(if ch.is_ascii_graphic() || ch == ' ' {
            ch
        } else {
            '?'
        });
    }
}

pub(super) fn render(entries: &[Entry], selected: usize, status: Option<Result<(), Status>>) {
    system::with_stdout(|out| {
        let (cols, rows) = out
            .current_mode()
            .ok()
            .flatten()
            .map_or((80, 25), |m| (m.columns(), m.rows()));
        let width = cols.saturating_sub(1);
        let _ = out.set_color(Color::LightGray, Color::Black);
        let _ = out.clear();
        let _ = out.enable_cursor(false);
        let groups = Groups::new(entries);
        let ranges = groups.ranges();
        let half = width / 2;
        let split = width >= 60 && !ranges[0].is_empty() && !ranges[1].is_empty();
        let active = ranges
            .iter()
            .position(|r| r.contains(&selected))
            .unwrap_or(0);
        let compact = rows < 12;
        let bottom = rows.saturating_sub(2);
        line(out, 0, 0, width, " EFI Swiss Knife ", true);
        let system_y = bottom.saturating_sub((ranges[2].len() + 1).min(bottom / 2));
        for (group, range) in ranges.into_iter().enumerate() {
            if range.is_empty()
                || (compact && group != active)
                || (!split
                    && group < 2
                    && group != active.min(1)
                    && !(group == 0 && groups.tools_end == groups.boot))
            {
                continue;
            }
            let (x, y, w, end) = if compact {
                (0, 1, width, bottom)
            } else if group == 2 {
                (0, system_y, width, bottom)
            } else {
                (
                    if split && group == 1 { half + 1 } else { 0 },
                    2,
                    if split { half } else { width },
                    system_y,
                )
            };
            let heading = ["Boot images", "Tools", "System"][group];
            let count = end.saturating_sub(y + 1);
            let offset = if range.contains(&selected) {
                (selected - range.start + 1).saturating_sub(count)
            } else {
                0
            };
            let title = if count < range.len() {
                format!(
                    "[ {heading} | {}-{}/{} ]",
                    offset + 1,
                    (offset + count).min(range.len()),
                    range.len()
                )
            } else {
                format!("[ {heading} ]")
            };
            line(out, x, y, w, &title, false);
            for (row, index) in range.skip(offset).take(count).enumerate() {
                let marker = if index == selected { '>' } else { ' ' };
                let label = &entries[index].label;
                let text = if group == 0 {
                    format!("{marker} {}. {label}", index + 1)
                } else {
                    format!("{marker} {label}")
                };
                line(out, x, y + row + 1, w, &text, index == selected);
            }
        }
        let detail = match status {
            Some(Err(s)) => failure_message(&entries[selected], s),
            Some(Ok(())) => "Boot entry returned to the menu.".into(),
            None => entries
                .get(selected)
                .and_then(Entry::tool_detail)
                .unwrap_or("")
                .into(),
        };
        line(out, 0, bottom, width, &detail, false);
        line(
            out,
            0,
            rows.saturating_sub(1),
            width,
            " Up/Down: select   Tab: section   Enter: open   Esc: exit ",
            true,
        );
    });
}
