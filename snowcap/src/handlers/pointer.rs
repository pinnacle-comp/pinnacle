use iced::mouse::{Interaction, ScrollDelta};
use smithay_client_toolkit::{
    delegate_pointer,
    reexports::{
        client::{
            Connection, QueueHandle,
            protocol::wl_pointer::{AxisSource, WlPointer},
        },
        protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape,
    },
    seat::pointer::{PointerEvent, PointerEventKind, PointerHandler},
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
            if let PointerEventKind::Enter { serial } = &event.kind {
                self.pointer_focus = Some(event.surface.clone());
                self.last_pointer_enter_serial = Some(*serial);
            }
            if let PointerEventKind::Leave { serial: _ } = &event.kind {
                self.pointer_focus = None;
            }

            let Some(surface) = self.find_surface_mut(&event.surface) else {
                continue;
            };

            let iced_event = match event.kind {
                PointerEventKind::Enter { serial } => {
                    surface.pointer_location = Some(event.position);
                    surface.focus_serial = Some(serial);
                    iced::Event::Mouse(iced::mouse::Event::CursorEntered)
                }
                PointerEventKind::Leave { serial: _ } => {
                    surface.pointer_location = None;
                    iced::Event::Mouse(iced::mouse::Event::CursorLeft)
                }
                PointerEventKind::Motion { time: _ } => {
                    surface.pointer_location = Some(event.position);
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
                    serial,
                } => {
                    surface.focus_serial = Some(serial);
                    iced::Event::Mouse(iced::mouse::Event::ButtonPressed(button_to_iced_button(
                        button,
                    )))
                }
                PointerEventKind::Release {
                    time: _,
                    button,
                    serial,
                } => {
                    surface.focus_serial = Some(serial);
                    iced::Event::Mouse(iced::mouse::Event::ButtonReleased(button_to_iced_button(
                        button,
                    )))
                }
                PointerEventKind::Axis {
                    time: _,
                    horizontal,
                    vertical,
                    source,
                } => {
                    // Values are negated because they're backwards otherwise
                    let delta = match source {
                        Some(AxisSource::Wheel | AxisSource::WheelTilt) => ScrollDelta::Lines {
                            x: -horizontal.value120 as f32 / 120.0,
                            y: -vertical.value120 as f32 / 120.0,
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

            surface.widgets.queue_event(iced_event);
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

pub fn iced_interaction_to_shape(interaction: Interaction) -> Option<Shape> {
    let shape = match interaction {
        Interaction::Hidden => return None,
        Interaction::None | Interaction::Idle => Shape::Default,
        Interaction::ContextMenu => Shape::ContextMenu,
        Interaction::Help => Shape::Help,
        Interaction::Pointer => Shape::Pointer,
        Interaction::Progress => Shape::Progress,
        Interaction::Wait => Shape::Wait,
        Interaction::Cell => Shape::Cell,
        Interaction::Crosshair => Shape::Crosshair,
        Interaction::Text => Shape::Text,
        Interaction::Alias => Shape::Alias,
        Interaction::Copy => Shape::Copy,
        Interaction::Move => Shape::Move,
        Interaction::NoDrop => Shape::NoDrop,
        Interaction::NotAllowed => Shape::NotAllowed,
        Interaction::Grab => Shape::Grab,
        Interaction::Grabbing => Shape::Grabbing,
        Interaction::ResizingHorizontally => Shape::EwResize,
        Interaction::ResizingVertically => Shape::NsResize,
        Interaction::ResizingDiagonallyUp => Shape::NeswResize,
        Interaction::ResizingDiagonallyDown => Shape::NwseResize,
        Interaction::ResizingColumn => Shape::ColResize,
        Interaction::ResizingRow => Shape::RowResize,
        Interaction::AllScroll => Shape::AllScroll,
        Interaction::ZoomIn => Shape::ZoomIn,
        Interaction::ZoomOut => Shape::ZoomOut,
    };

    Some(shape)
}
