use uefi::boot::{self, ScopedProtocol};
use uefi::proto::ProtocolPointer;
use uefi::proto::console::pointer::{AbsolutePointer, Pointer};
use uefi::{Event, Result};

const SUBPIXELS: i64 = 256;
const PIXELS_PER_MM: i64 = 5;

enum Device {
    Relative(ScopedProtocol<Pointer>),
    Absolute(ScopedProtocol<AbsolutePointer>),
}

pub struct Mouse {
    device: Device,
    bounds: (usize, usize),
    position: (i64, i64),
}

pub struct State {
    pub point: (usize, usize),
    pub pressed: bool,
}

impl Mouse {
    pub fn open(bounds: (usize, usize)) -> Option<(Self, Event)> {
        let relative = open::<Pointer>().and_then(|device| {
            let event = device.wait_for_input_event().ok()?;
            Some((Device::Relative(device), event))
        });
        let (device, event) = relative.or_else(|| {
            let device = open::<AbsolutePointer>()?;
            let event = device.wait_for_input_event().ok()?;
            Some((Device::Absolute(device), event))
        })?;
        let position = (
            bounds.0 as i64 / 2 * SUBPIXELS,
            bounds.1 as i64 / 2 * SUBPIXELS,
        );
        Some((
            Self {
                device,
                bounds,
                position,
            },
            event,
        ))
    }

    pub fn read(&mut self) -> Result<Option<State>> {
        let (width, height) = self.bounds;
        let pressed = match &mut self.device {
            Device::Relative(device) => {
                let Some(state) = device.read_state()? else {
                    return Ok(None);
                };
                let mode = device.mode();
                self.position.0 = move_axis(
                    self.position.0,
                    state.relative_movement_x,
                    mode.resolution_x,
                    width,
                );
                self.position.1 = move_axis(
                    self.position.1,
                    state.relative_movement_y,
                    mode.resolution_y,
                    height,
                );
                bool::from(state.left_button)
            }
            Device::Absolute(device) => {
                let Some(state) = device.read_state()? else {
                    return Ok(None);
                };
                let mode = device.mode();
                self.position.0 = absolute_axis(
                    state.current_x,
                    mode.absolute_min_x,
                    mode.absolute_max_x,
                    width,
                );
                self.position.1 = absolute_axis(
                    state.current_y,
                    mode.absolute_min_y,
                    mode.absolute_max_y,
                    height,
                );
                state.active_buttons & 1 != 0
            }
        };

        Ok(Some(State {
            point: (
                (self.position.0 / SUBPIXELS) as usize,
                (self.position.1 / SUBPIXELS) as usize,
            ),
            pressed,
        }))
    }
}

fn open<P: ProtocolPointer>() -> Option<ScopedProtocol<P>> {
    let handle = boot::get_handle_for_protocol::<P>().ok()?;
    boot::open_protocol_exclusive::<P>(handle).ok()
}

fn move_axis(position: i64, counts: i32, resolution: u64, limit: usize) -> i64 {
    let counts = i64::from(counts) * SUBPIXELS;
    let device_delta = match resolution {
        0 => counts,
        _ => counts * PIXELS_PER_MM / resolution.min(i64::MAX as u64) as i64,
    };
    (position + device_delta).clamp(0, (limit - 1) as i64 * SUBPIXELS)
}

fn absolute_axis(value: u64, min: u64, max: u64, limit: usize) -> i64 {
    if max <= min {
        return 0;
    }
    let position = u128::from(value.clamp(min, max) - min);
    let pixels = position * (limit - 1) as u128 / u128::from(max - min);
    pixels as i64 * SUBPIXELS
}
