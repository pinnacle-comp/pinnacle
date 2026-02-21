use smithay_client_toolkit::{
    delegate_touch,
    reexports::client::protocol::{wl_surface::WlSurface, wl_touch::WlTouch},
    seat::touch::TouchHandler,
};

use crate::state::State;

#[derive(Clone, Debug)]
pub struct ActiveTouch {
    id: i32,
    touch: WlTouch,
    surface: WlSurface,
    last_pos: (f64, f64),
}

impl TouchHandler for State {
    fn down(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &smithay_client_toolkit::reexports::client::QueueHandle<Self>,
        touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
        _serial: u32,
        _time: u32,
        surface: smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface,
        id: i32,
        position: (f64, f64),
    ) {
        self.add_active_touch(ActiveTouch {
            id,
            touch: touch.clone(),
            surface,
            last_pos: position,
        });
    }

    fn motion(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &smithay_client_toolkit::reexports::client::QueueHandle<Self>,
        _touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
        _time: u32,
        id: i32,
        position: (f64, f64),
    ) {
        self.active_touch_motion(id, position);
    }

    fn up(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &smithay_client_toolkit::reexports::client::QueueHandle<Self>,
        _touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
        _serial: u32,
        _time: u32,
        id: i32,
    ) {
        self.active_touch_up(id);
    }

    fn cancel(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &smithay_client_toolkit::reexports::client::QueueHandle<Self>,
        touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
    ) {
        self.cancel_all_touch(touch);
    }

    fn shape(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &smithay_client_toolkit::reexports::client::QueueHandle<Self>,
        _touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
        _id: i32,
        _major: f64,
        _minor: f64,
    ) {
    }

    fn orientation(
        &mut self,
        _conn: &smithay_client_toolkit::reexports::client::Connection,
        _qh: &smithay_client_toolkit::reexports::client::QueueHandle<Self>,
        _touch: &smithay_client_toolkit::reexports::client::protocol::wl_touch::WlTouch,
        _id: i32,
        _orientation: f64,
    ) {
    }
}
delegate_touch!(State);

impl State {
    fn add_active_touch(&mut self, touch: ActiveTouch) {
        let Some(surface) = self.find_surface_mut(&touch.surface) else {
            tracing::warn!("surface not found for touch #{}", touch.id);
            return;
        };

        let id = iced::touch::Finger(touch.id as u64);
        let position = iced::Point {
            x: touch.last_pos.0 as f32,
            y: touch.last_pos.1 as f32,
        };

        let event = iced::Event::Touch(iced::touch::Event::FingerPressed { id, position });

        surface.pointer_location = Some(touch.last_pos);
        surface.widgets.queue_event(event);
        self.active_touches.push(touch);
    }

    fn active_touch_motion(&mut self, id: i32, position: (f64, f64)) {
        let touch: ActiveTouch = {
            let Some(touch) = self.active_touches.iter_mut().find(|t| t.id == id) else {
                tracing::warn!("No active touch with id: #{id}");
                return;
            };

            touch.last_pos = position;

            touch.clone()
        };

        let Some(surface) = self.find_surface_mut(&touch.surface) else {
            tracing::warn!("Could not find surface for touch #{id}");
            self.active_touches.retain(|t| t.id != id);
            return;
        };

        let id = iced::touch::Finger(touch.id as u64);
        let position = iced::Point {
            x: touch.last_pos.0 as f32,
            y: touch.last_pos.1 as f32,
        };

        let event = iced::Event::Touch(iced::touch::Event::FingerMoved { id, position });

        surface.pointer_location = Some(touch.last_pos);
        surface.widgets.queue_event(event);
    }

    fn active_touch_up(&mut self, id: i32) {
        let Some(touch) = self.active_touches.extract_if(.., |t| t.id == id).next() else {
            tracing::warn!("No active touch with id: #{id}");
            return;
        };

        let Some(surface) = self.find_surface_mut(&touch.surface) else {
            tracing::warn!("Could not find surface for touch #{id}");
            return;
        };

        let id = iced::touch::Finger(touch.id as u64);
        let position = iced::Point {
            x: touch.last_pos.0 as f32,
            y: touch.last_pos.1 as f32,
        };

        let event = iced::Event::Touch(iced::touch::Event::FingerLifted { id, position });

        surface.pointer_location = Some(touch.last_pos);
        surface.widgets.queue_event(event);
    }

    pub(crate) fn cancel_all_touch(&mut self, touch: &WlTouch) {
        let to_cancel = self
            .active_touches
            .extract_if(.., |t| &t.touch == touch)
            .collect::<Vec<_>>();

        for touch in to_cancel {
            let Some(surface) = self.find_surface_mut(&touch.surface) else {
                continue;
            };

            let id = iced::touch::Finger(touch.id as u64);
            let position = iced::Point {
                x: touch.last_pos.0 as f32,
                y: touch.last_pos.1 as f32,
            };

            let event = iced::Event::Touch(iced::touch::Event::FingerLost { id, position });

            surface.pointer_location = Some(touch.last_pos);
            surface.widgets.queue_event(event);
        }
    }

    pub(crate) fn flush_touch_for_surface(&mut self, wl_surface: &WlSurface) {
        self.active_touches.retain(|t| &t.surface != wl_surface);
    }
}
