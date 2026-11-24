mod canvas;
mod cursor;

use uefi::boot::{self, OpenProtocolAttributes, OpenProtocolParams, ScopedProtocol};
use uefi::proto::console::gop::{BltOp, BltRegion, GraphicsOutput};
use uefi::{Result, Status};

use self::canvas::{Canvas, Face};
use self::cursor::Cursor;
use super::Groups;
use crate::actions::Action;
use crate::entries::{Entry, Target};
use crate::iso::artwork::ICON_SIZE;

// shadcn/ui neutral dark tokens converted to sRGB for GOP.
const BACKGROUND: u32 = 0x0a0a0a;
const CARD: u32 = 0x171717;
const BORDER: u32 = 0x2e2e2e; // 10% white over CARD.
const ACCENT: u32 = 0x262626;
const FOREGROUND: u32 = 0xfafafa;
const MUTED_FOREGROUND: u32 = 0xa1a1a1;
const RING: u32 = 0x737373;
const DESTRUCTIVE: u32 = 0xff6467;
const LIST_TOP: usize = 12;
const BOTTOM_PADDING: usize = 24;
const ROW_INSET: usize = 12;
const ROW_GAP: usize = 4;
const GAP: usize = 12;
const HEADER: usize = 36;
const ACTION_SIZE: usize = 38;
const ACTION_GAP: usize = 16;

pub struct Graphics {
    output: ScopedProtocol<GraphicsOutput>,
    canvas: Canvas,
    origin: (usize, usize),
    cards: [(usize, usize, usize, usize); 2],
    rows: alloc::vec::Vec<(usize, usize, usize, usize)>,
    bounds: (usize, usize),
    cursor: Option<Cursor>,
}

impl Graphics {
    pub fn open(groups: Groups) -> Option<Self> {
        let handle = boot::get_handle_for_protocol::<GraphicsOutput>().ok()?;
        // Keep the console driver attached to GOP so it can render after we
        // leave the picker. We issue synchronous blits without changing mode.
        let mut output = unsafe {
            boot::open_protocol::<GraphicsOutput>(
                OpenProtocolParams {
                    handle,
                    agent: boot::image_handle(),
                    controller: None,
                },
                OpenProtocolAttributes::GetProtocol,
            )
        }
        .ok()?;
        let (width, height) = output.current_mode_info().resolution();
        if width < 440 || height < 400 {
            return None;
        }

        let width_available = (width - 32).min(900) & !1;
        let tools = groups.tools_end - groups.boot;
        let side_by_side = width_available >= 710 && groups.boot > 0 && tools > 0;
        let card_count = usize::from(groups.boot > 0) + usize::from(tools > 0);
        let bands = if side_by_side { 1 } else { card_count };
        let vertical_rows = if side_by_side {
            groups.boot.max(tools)
        } else {
            groups.tools_end
        };
        let fixed =
            LIST_TOP + BOTTOM_PADDING + 24 + bands * (HEADER + ROW_INSET + GAP) + ACTION_SIZE;
        let row_height = (height.checked_sub(fixed)? / vertical_rows.max(1)).min(38);
        if vertical_rows > 0 && row_height < 26 {
            return None;
        }
        let mut cards = [(0, 0, 0, 0); 2];
        let mut rows = alloc::vec::Vec::with_capacity(groups.total);
        let mut bottom = LIST_TOP;
        let inner = width_available - 2 * ROW_INSET;
        let column_width = (inner - GAP) / 2;
        for (group, range) in groups.ranges().into_iter().take(2).enumerate() {
            if range.is_empty() {
                continue;
            }
            let (x, y, w) = if side_by_side {
                if group == 0 {
                    (ROW_INSET, LIST_TOP, column_width)
                } else {
                    (ROW_INSET + column_width + GAP, LIST_TOP, column_width)
                }
            } else {
                (ROW_INSET, bottom, inner)
            };
            let count = if side_by_side {
                vertical_rows
            } else {
                range.len()
            };
            let h = HEADER + count * row_height + ROW_INSET;
            cards[group] = (x, y, w, h);
            for index in 0..range.len() {
                rows.push((
                    x + ROW_INSET,
                    y + HEADER + index * row_height,
                    w - 2 * ROW_INSET,
                    row_height - ROW_GAP,
                ));
            }
            bottom = bottom.max(y + h + GAP);
        }
        let count = groups.total - groups.tools_end;
        let buttons_width = count * ACTION_SIZE + count.saturating_sub(1) * ACTION_GAP;
        let buttons_start = (width_available - buttons_width) / 2;
        for index in 0..count {
            rows.push((
                buttons_start + index * (ACTION_SIZE + ACTION_GAP),
                bottom,
                ACTION_SIZE,
                ACTION_SIZE,
            ));
        }
        let panel_height = bottom + ACTION_SIZE + BOTTOM_PADDING;
        let canvas = Canvas::new(width_available, panel_height)?;
        let origin = ((width - canvas.width) / 2, (height - canvas.height) / 2);

        output
            .blt(BltOp::VideoFill {
                color: BACKGROUND.into(),
                dest: (0, 0),
                dims: (width, height),
            })
            .ok()?;
        Some(Self {
            output,
            canvas,
            origin,
            cards,
            rows,
            bounds: (width, height),
            cursor: None,
        })
    }

    pub fn bounds(&self) -> (usize, usize) {
        self.bounds
    }

    pub fn clear_for_handoff(&mut self) -> Result {
        self.cursor = None;
        self.output.blt(BltOp::VideoFill {
            color: 0u32.into(),
            dest: (0, 0),
            dims: self.bounds,
        })
    }

    pub fn entry_at(&self, (x, y): (usize, usize)) -> Option<usize> {
        let x = x.checked_sub(self.origin.0)?;
        let y = y.checked_sub(self.origin.1)?;
        self.rows.iter().position(|&(left, top, w, h)| {
            (left..left + w).contains(&x) && (top..top + h).contains(&y)
        })
    }

    pub fn show_cursor(&mut self, point: (usize, usize)) -> Result {
        self.hide_cursor()?;
        self.cursor = Some(Cursor::draw(&mut self.output, point, self.bounds)?);
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result {
        if let Some(cursor) = self.cursor.take() {
            cursor.erase(&mut self.output)?;
        }
        Ok(())
    }

    pub fn render(
        &mut self,
        entries: &[Entry],
        selected: usize,
        last_boot: Option<core::result::Result<(), Status>>,
    ) -> Result {
        self.hide_cursor()?;
        let canvas = &mut self.canvas;
        let (width, height) = (canvas.width, canvas.height);

        canvas.rect(0, 0, width, height, BACKGROUND);
        for (&(x, y, w, h), title) in self.cards.iter().zip(["Boot images", "Tools"]) {
            if w == 0 {
                continue;
            }
            canvas.rounded_rect((x, y), (w, h), 12, BORDER);
            canvas.rounded_rect((x + 1, y + 1), (w - 2, h - 2), 11, CARD);
            canvas.text(x + ROW_INSET + 4, y + 23, title, MUTED_FOREGROUND);
        }

        for (index, (entry, &(x, y, w, row_height))) in entries.iter().zip(&self.rows).enumerate() {
            let active = index == selected;
            let action = matches!(entry.target, Target::Action(_));
            if active || action {
                canvas.rounded_rect(
                    (x, y),
                    (w, row_height),
                    8,
                    if active { RING } else { BORDER },
                );
                canvas.rounded_rect(
                    (x + 1, y + 1),
                    (w - 2, row_height - 2),
                    7,
                    if active { ACCENT } else { CARD },
                );
            }

            let center = y + row_height / 2;
            if let Target::Action(action) = entry.target {
                render_action_icon(canvas, x + (w - 20) / 2, center - 10, action);
                continue;
            }
            let logo = entry.icon.as_deref();
            if let Some(logo) = logo {
                canvas.icon((x + 13, center - 9), ICON_SIZE, logo);
            } else if entry.tool_detail() == Some("EDK II") {
                let surface = if active { ACCENT } else { CARD };
                canvas.rounded_rect((x + 13, center - 7), (18, 14), 3, MUTED_FOREGROUND);
                canvas.rounded_rect((x + 14, center - 6), (16, 12), 2, surface);
                for (dx, dy) in [(17, 3), (18, 2), (19, 1), (18, 0), (17, -1)] {
                    canvas.rect(
                        x + dx,
                        center.checked_add_signed(dy).unwrap(),
                        1,
                        1,
                        FOREGROUND,
                    );
                }
                canvas.rect(x + 22, center + 3, 4, 1, FOREGROUND);
            } else if entry.is_tool() {
                let surface = if active { ACCENT } else { CARD };
                canvas.rounded_rect((x + 13, center - 6), (18, 10), 2, MUTED_FOREGROUND);
                canvas.rect(x + 15, center - 4, 14, 6, surface);
                for contact in [16, 21, 26] {
                    canvas.rect(x + contact, center + 4, 2, 2, MUTED_FOREGROUND);
                }
            } else {
                let surface = if active { ACCENT } else { CARD };
                canvas.rounded_rect((x + 16, center - 8), (12, 16), 3, MUTED_FOREGROUND);
                canvas.rounded_rect((x + 17, center - 7), (10, 14), 2, surface);
                canvas.rect(x + 19, center - 2, 6, 1, MUTED_FOREGROUND);
                canvas.rect(x + 19, center + 1, 6, 1, MUTED_FOREGROUND);
            }

            render_label(canvas, x + 45, center + 4, w - 57, entry);
        }

        let status_baseline = height - BOTTOM_PADDING + 13;
        match last_boot {
            Some(Ok(())) => canvas.text(
                24,
                status_baseline,
                "Boot entry returned to the menu.",
                MUTED_FOREGROUND,
            ),
            Some(Err(status)) => {
                let message = super::failure_message(&entries[selected], status);
                canvas.text(24, status_baseline, &message, DESTRUCTIVE);
            }
            None if matches!(entries[selected].target, Target::Action(_)) => {
                let label = &entries[selected].label;
                let label_width = canvas.text_width(label, Face::Regular);
                canvas.text(
                    (width - label_width) / 2,
                    status_baseline,
                    label,
                    MUTED_FOREGROUND,
                );
            }
            None => {}
        }

        self.output.blt(BltOp::BufferToVideo {
            buffer: &canvas.pixels,
            src: BltRegion::Full,
            dest: self.origin,
            dims: (width, height),
        })
    }
}

fn render_action_icon(canvas: &mut Canvas, x: usize, y: usize, action: Action) {
    let pixels: &[u8; 1600] = match action {
        Action::Local(_) => include_bytes!("../../../assets/lucide/hard-drive.rgba"),
        Action::Firmware => include_bytes!("../../../assets/lucide/settings.rgba"),
        Action::Exit => include_bytes!("../../../assets/lucide/log-out.rgba"),
        Action::Reboot => include_bytes!("../../../assets/lucide/rotate-cw.rgba"),
        Action::Shutdown => include_bytes!("../../../assets/lucide/power.rgba"),
    };
    canvas.icon((x, y), 20, pixels);
}

fn render_label(canvas: &mut Canvas, start: usize, baseline: usize, width: usize, entry: &Entry) {
    let detail = match &entry.target {
        Target::Iso(_, path) => path.rsplit('/').next(),
        _ => entry.tool_detail(),
    };
    let measure = |canvas: &Canvas, text: &str| canvas.text_width(text, Face::Regular);
    let parentheses_width = measure(canvas, " ()");
    let reserved = detail.map_or(0, |name| {
        (measure(canvas, name) + parentheses_width).min(width / 2)
    });

    let (label, truncated) = canvas.fit_label(&entry.label, width - reserved);
    let mut x = start;
    canvas.text(x, baseline, label, FOREGROUND);
    x += measure(canvas, label);
    if truncated {
        canvas.text(x, baseline, "…", FOREGROUND);
        x += measure(canvas, "…");
    }

    if let Some(detail) = detail {
        let available = width.saturating_sub(x - start + parentheses_width);
        let (detail, truncated) = canvas.fit_label(detail, available);
        canvas.text(x, baseline, " (", MUTED_FOREGROUND);
        x += measure(canvas, " (");
        canvas.text(x, baseline, detail, MUTED_FOREGROUND);
        x += measure(canvas, detail);
        if truncated {
            canvas.text(x, baseline, "…", MUTED_FOREGROUND);
            x += measure(canvas, "…");
        }
        canvas.text(x, baseline, ")", MUTED_FOREGROUND);
    }
}
