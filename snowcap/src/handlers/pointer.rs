use iced::mouse::ScrollDelta;
use smithay_client_toolkit::{
    delegate_pointer,
    reexports::client::{
        protocol::wl_pointer::{AxisSource, WlPointer},
        Connection, QueueHandle,
    },
    seat::pointer::{PointerEvent, PointerEventKind, PointerHandler},
    shell::WaylandSurface,
};

use crate::state::State;

impl PointerHandler for State {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            let Some(layer) = self
                .layers
                .iter_mut()
                .find(|sn_layer| sn_layer.layer.wl_surface() == &event.surface)
            else {
                continue;
            };

            let iced_event = match event.kind {
                PointerEventKind::Enter { serial: _ } => {
                    layer.pointer_location = Some(event.position);
                    iced::Event::Mouse(iced::mouse::Event::CursorEntered)
                }
                PointerEventKind::Leave { serial: _ } => {
                    layer.pointer_location = None;
                    iced::Event::Mouse(iced::mouse::Event::CursorLeft)
                }
                PointerEventKind::Motion { time: _ } => {
                    layer.pointer_location = Some(event.position);
                    iced::Event::Mouse(iced::mouse::Event::CursorMoved {
                        position: iced::Point {
                            x: event.position.0 as f32,
                            y: event.position.1 as f32,
                        },
                    })
                }
                PointerEventKind::Press {
                    time: _,
                    button,
                    serial: _,
                } => iced::Event::Mouse(iced::mouse::Event::ButtonPressed(button_to_iced_button(
                    button,
                ))),
                PointerEventKind::Release {
                    time: _,
                    button,
                    serial: _,
                } => iced::Event::Mouse(iced::mouse::Event::ButtonReleased(button_to_iced_button(
                    button,
                ))),
                PointerEventKind::Axis {
                    time: _,
                    horizontal,
                    vertical,
                    source,
                } => {
                    // Values are negated because they're backwards otherwise
                    let delta = match source {
                        Some(AxisSource::Wheel | AxisSource::WheelTilt) => ScrollDelta::Lines {
                            x: -horizontal.discrete as f32,
                            y: -vertical.discrete as f32,
                        },
                        Some(AxisSource::Finger | AxisSource::Continuous) => ScrollDelta::Pixels {
                            x: -horizontal.absolute as f32,
                            y: -vertical.absolute as f32,
                        },
                        // TODO: continue here or default to lines? prolly should
                        // look at the protocol docs
                        _ => continue,
                    };
                    iced::Event::Mouse(iced::mouse::Event::WheelScrolled { delta })
                }
            };

            layer.widgets.queue_event(iced_event);
        }
    }
}
delegate_pointer!(State);

fn button_to_iced_button(button: u32) -> iced::mouse::Button {
    match button {
        0x110 => iced::mouse::Button::Left,
        0x111 => iced::mouse::Button::Right,
        0x112 => iced::mouse::Button::Middle,
        0x115 => iced::mouse::Button::Forward,
        0x116 => iced::mouse::Button::Back,
        button => iced::mouse::Button::Other(button as u16),
    }
}
