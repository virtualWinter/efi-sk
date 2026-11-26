mod graphics;
mod pointer;

use alloc::vec;

use core::fmt::Write;
use uefi::boot;
use uefi::proto::console::text::{Color, Key, ScanCode};
use uefi::{Status, system};

use crate::entries::{Entry, Target};

fn failure_message(entry: &Entry, status: Status) -> alloc::string::String {
    if entry.is_keytool()
        && status == Status::UNSUPPORTED
        && !crate::system_info::has_secure_boot_setup()
    {
        return "KeyTool requires firmware with Secure Boot support.".into();
    }
    alloc::format!("Could not boot the selected entry: {status:?}")
}

#[derive(Clone, Copy)]
struct Groups {
    boot: usize,
    tools_end: usize,
    total: usize,
}

impl Groups {
    fn new(entries: &[Entry]) -> Self {
        Self {
            boot: entries
                .iter()
                .take_while(|entry| !entry.is_tool() && !matches!(entry.target, Target::Action(_)))
                .count(),
            tools_end: entries
                .iter()
                .take_while(|entry| !matches!(entry.target, Target::Action(_)))
                .count(),
            total: entries.len(),
        }
    }

    fn ranges(self) -> [core::ops::Range<usize>; 3] {
        [
            0..self.boot,
            self.boot..self.tools_end,
            self.tools_end..self.total,
        ]
    }

    fn navigate(self, selected: usize, key: ScanCode) -> usize {
        let ranges = self.ranges();
        let group = ranges
            .iter()
            .position(|range| range.contains(&selected))
            .unwrap_or(0);
        let range = &ranges[group];
        match key {
            ScanCode::UP if group < 2 => {
                range.start + (selected - range.start + range.len() - 1) % range.len()
            }
            ScanCode::DOWN if group < 2 => range.start + (selected - range.start + 1) % range.len(),
            ScanCode::LEFT if group == 2 => {
                range.start + (selected - range.start + range.len() - 1) % range.len()
            }
            ScanCode::RIGHT if group == 2 => {
                range.start + (selected - range.start + 1) % range.len()
            }
            ScanCode::UP if group == 2 => ranges[..2]
                .iter()
                .rev()
                .find(|r| !r.is_empty())
                .map_or(selected, |r| r.start),
            _ => selected,
        }
    }

    fn tab(self, selected: usize) -> usize {
        let ranges = self.ranges();
        let group = ranges
            .iter()
            .position(|range| range.contains(&selected))
            .unwrap_or(0);
        (1..=3)
            .map(|step| &ranges[(group + step) % 3])
            .find(|r| !r.is_empty())
            .map_or(selected, |r| r.start)
    }
}

pub fn select(
    entries: &[Entry],
    selected: &mut usize,
    mut last_boot: Option<Result<(), Status>>,
) -> Result<bool, Status> {
    let groups = Groups::new(entries);
    let mut graphics = if cfg!(feature = "text-mode") {
        None
    } else {
        graphics::Graphics::open(groups)
    };
    let key_event =
        system::with_stdin(|input| input.wait_for_key_event()).map_err(|error| error.status())?;
    render(&mut graphics, entries, *selected, last_boot);

    let mut events = vec![key_event];
    let mut mouse = graphics
        .as_ref()
        .and_then(|display| pointer::Mouse::open(display.bounds()))
        .map(|(mouse, event)| {
            events.push(event);
            mouse
        });
    let mut cursor = None;
    let mut pressed = false;
    let mut pressed_entry = None;

    loop {
        let event = match boot::wait_for_event(&events) {
            Ok(event) => event,
            Err(error) if error.data() == &Some(1) => {
                mouse = None;
                events.truncate(1);
                continue;
            }
            Err(error) => return Err(error.status()),
        };

        if event == 1 {
            let Some(device) = &mut mouse else {
                continue;
            };
            let state = match device.read() {
                Ok(Some(state)) => state,
                Ok(None) => continue,
                Err(_) => {
                    mouse = None;
                    events.truncate(1);
                    continue;
                }
            };
            let hovered = graphics
                .as_ref()
                .and_then(|display| display.entry_at(state.point));

            if state.pressed && !pressed {
                pressed_entry = hovered;
            } else if !state.pressed
                && pressed
                && let Some(entry) = pressed_entry.take().filter(|entry| Some(*entry) == hovered)
            {
                *selected = entry;
                break;
            }
            pressed = state.pressed;

            if let Some(entry) = hovered.filter(|entry| *entry != *selected) {
                *selected = entry;
                last_boot = None;
                render(&mut graphics, entries, *selected, last_boot);
            }
            cursor = Some(state.point);
            if !draw_cursor(&mut graphics, cursor) {
                mouse = None;
                events.truncate(1);
                cursor = None;
            }
            continue;
        }

        let key = system::with_stdin(|input| input.read_key()).map_err(|error| error.status())?;
        let next = match key {
            Some(Key::Special(ScanCode::ESCAPE)) => {
                prepare_console(graphics);
                return Ok(false);
            }
            Some(Key::Special(key)) => groups.navigate(*selected, key),
            Some(Key::Printable(key)) => {
                let key = char::from(key);
                if matches!(key, '\r' | '\n') {
                    break;
                }
                if key == '\t' {
                    groups.tab(*selected)
                } else if let Some(number) = key.to_digit(10) {
                    let number = number as usize;
                    if (1..=groups.boot).contains(&number) {
                        *selected = number - 1;
                        break;
                    }
                    continue;
                } else {
                    continue;
                }
            }
            _ => continue,
        };

        if next != *selected {
            *selected = next;
            last_boot = None;
            render(&mut graphics, entries, *selected, last_boot);
            if !draw_cursor(&mut graphics, cursor) {
                mouse = None;
                events.truncate(1);
                cursor = None;
            }
        }
    }
    prepare_console(graphics);
    Ok(true)
}

fn prepare_console(mut graphics: Option<graphics::Graphics>) {
    if let Some(display) = &mut graphics {
        let _ = display.clear_for_handoff();
    }
    drop(graphics);
    system::with_stdout(|out| {
        let _ = out.set_color(Color::LightGray, Color::Black);
        let _ = out.clear();
        let _ = out.set_cursor_position(0, 0);
        let _ = out.enable_cursor(true);
    });
}

fn draw_cursor(graphics: &mut Option<graphics::Graphics>, point: Option<(usize, usize)>) -> bool {
    graphics
        .as_mut()
        .is_some_and(|display| point.is_none_or(|point| display.show_cursor(point).is_ok()))
}

fn render(
    graphics: &mut Option<graphics::Graphics>,
    entries: &[Entry],
    selected: usize,
    last_boot: Option<Result<(), Status>>,
) {
    if let Some(display) = graphics {
        if display.render(entries, selected, last_boot).is_ok() {
            return;
        }
        *graphics = None;
    }
    render_text(entries, selected, last_boot);
}

fn render_text(entries: &[Entry], selected: usize, last_boot: Option<Result<(), Status>>) {
    system::with_stdout(|out| {
        let _ = out.set_color(Color::LightGray, Color::Black);
        let _ = out.clear();

        let groups = Groups::new(entries);
        for (index, entry) in entries.iter().enumerate() {
            if index == 0 && groups.boot > 0 {
                let _ = writeln!(out, "  Boot images");
            }
            if index == groups.boot && groups.boot < groups.tools_end {
                let _ = writeln!(out, "\n  Tools");
            }
            if index == groups.tools_end {
                let _ = writeln!(out, "\n  System");
            }
            let number = index + 1;
            let label = &entry.label;
            let active = index == selected;
            if active {
                let _ = out.set_color(Color::Black, Color::LightGray);
            }
            if matches!(entry.target, Target::Action(_)) {
                let marker = if active { '>' } else { ' ' };
                let _ = write!(out, "  {marker} {label}");
            } else if let Some(detail) = entry.tool_detail() {
                let marker = if active { '>' } else { ' ' };
                let _ = write!(out, "  {marker} {label} ({detail})");
            } else if active {
                let _ = write!(out, "  > {number}. {label:<60} ");
            } else {
                let _ = write!(out, "    {number}. {label}");
            }
            if active {
                let _ = out.set_color(Color::LightGray, Color::Black);
            }
            let _ = writeln!(out);
        }

        match last_boot {
            Some(Ok(())) => {
                let _ = writeln!(out, "\n  Boot entry returned to the menu.");
            }
            Some(Err(status)) => {
                let _ = out.set_color(Color::LightRed, Color::Black);
                let _ = writeln!(out, "\n  {}", failure_message(&entries[selected], status));
                let _ = out.set_color(Color::LightGray, Color::Black);
            }
            None => {}
        }
    });
}
