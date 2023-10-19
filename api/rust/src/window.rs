use crate::{
    msg::{FullscreenOrMaximized, Msg, Request, RequestResponse, WindowId},
    request, send_msg,
};

pub struct Window;

impl Window {
    pub fn get_by_class<'a>(&self, class: &'a str) -> impl Iterator<Item = WindowHandle> + 'a {
        self.get_all()
            .filter(|win| win.class().as_deref() == Some(class))
    }

    pub fn get_focused(&self) -> Option<WindowHandle> {
        self.get_all()
            .find(|win| win.focused().is_some_and(|focused| focused))
    }

    pub fn get_all(&self) -> impl Iterator<Item = WindowHandle> {
        let RequestResponse::Windows { window_ids } = request(Request::GetWindows) else {
            unreachable!()
        };

        window_ids.into_iter().map(WindowHandle)
    }
}

pub struct WindowHandle(WindowId);

impl WindowHandle {
    pub fn toggle_floating(&self) {
        send_msg(Msg::ToggleFloating { window_id: self.0 }).unwrap();
    }

    pub fn toggle_fullscreen(&self) {
        send_msg(Msg::ToggleFullscreen { window_id: self.0 }).unwrap();
    }

    pub fn toggle_maximized(&self) {
        send_msg(Msg::ToggleMaximized { window_id: self.0 }).unwrap();
    }

    pub fn set_size(&self, width: Option<i32>, height: Option<i32>) {
        send_msg(Msg::SetWindowSize {
            window_id: self.0,
            width,
            height,
        })
        .unwrap();
    }

    pub fn close(&self) {
        send_msg(Msg::CloseWindow { window_id: self.0 }).unwrap();
    }

    pub fn size(&self) -> Option<(i32, i32)> {
        let RequestResponse::WindowProps { size, .. } =
            request(Request::GetWindowProps { window_id: self.0 })
        else {
            unreachable!()
        };

        size
    }

    pub fn loc(&self) -> Option<(i32, i32)> {
        let RequestResponse::WindowProps { loc, .. } =
            request(Request::GetWindowProps { window_id: self.0 })
        else {
            unreachable!()
        };

        loc
    }

    pub fn class(&self) -> Option<String> {
        let RequestResponse::WindowProps { class, .. } =
            request(Request::GetWindowProps { window_id: self.0 })
        else {
            unreachable!()
        };

        class
    }

    pub fn title(&self) -> Option<String> {
        let RequestResponse::WindowProps { title, .. } =
            request(Request::GetWindowProps { window_id: self.0 })
        else {
            unreachable!()
        };

        title
    }

    pub fn floating(&self) -> Option<bool> {
        let RequestResponse::WindowProps { floating, .. } =
            request(Request::GetWindowProps { window_id: self.0 })
        else {
            unreachable!()
        };

        floating
    }

    pub fn maximized(&self) -> Option<bool> {
        let RequestResponse::WindowProps {
            fullscreen_or_maximized,
            ..
        } = request(Request::GetWindowProps { window_id: self.0 })
        else {
            unreachable!()
        };

        fullscreen_or_maximized.map(|fullscreen_or_maximized| {
            matches!(fullscreen_or_maximized, FullscreenOrMaximized::Maximized)
        })
    }

    pub fn fullscreen(&self) -> Option<bool> {
        let RequestResponse::WindowProps {
            fullscreen_or_maximized,
            ..
        } = request(Request::GetWindowProps { window_id: self.0 })
        else {
            unreachable!()
        };

        fullscreen_or_maximized.map(|fullscreen_or_maximized| {
            matches!(fullscreen_or_maximized, FullscreenOrMaximized::Fullscreen)
        })
    }

    pub fn focused(&self) -> Option<bool> {
        let RequestResponse::WindowProps { focused, .. } =
            request(Request::GetWindowProps { window_id: self.0 })
        else {
            unreachable!()
        };

        focused
    }
}
