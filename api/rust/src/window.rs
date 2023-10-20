pub mod rules;

use crate::{
    input::MouseButton,
    msg::{Msg, Request, RequestResponse},
    request, send_msg,
    tag::TagHandle,
};

use self::rules::WindowRules;

/// A unique identifier for each window.
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WindowId {
    /// A config API returned an invalid window. It should be using this variant.
    None,
    /// A valid window id.
    #[serde(untagged)]
    Some(u32),
}

#[derive(Clone, Copy)]
pub struct Window {
    pub rules: WindowRules,
}

impl Window {
    pub fn get_by_class<'a>(&self, class: &'a str) -> impl Iterator<Item = WindowHandle> + 'a {
        self.get_all()
            .filter(|win| win.properties().class.as_deref() == Some(class))
    }

    pub fn get_focused(&self) -> Option<WindowHandle> {
        self.get_all()
            .find(|win| win.properties().focused.is_some_and(|focused| focused))
    }

    pub fn get_all(&self) -> impl Iterator<Item = WindowHandle> {
        let RequestResponse::Windows { window_ids } = request(Request::GetWindows) else {
            unreachable!()
        };

        window_ids.into_iter().map(WindowHandle)
    }

    pub fn begin_move(&self, button: MouseButton) {
        let msg = Msg::WindowMoveGrab {
            button: button as u32,
        };

        send_msg(msg).unwrap();
    }

    pub fn begin_resize(&self, button: MouseButton) {
        let msg = Msg::WindowResizeGrab {
            button: button as u32,
        };

        send_msg(msg).unwrap();
    }
}

pub struct WindowHandle(WindowId);

#[derive(Debug)]
pub struct WindowProperties {
    pub size: Option<(i32, i32)>,
    pub loc: Option<(i32, i32)>,
    pub class: Option<String>,
    pub title: Option<String>,
    pub focused: Option<bool>,
    pub floating: Option<bool>,
    pub fullscreen_or_maximized: Option<FullscreenOrMaximized>,
}

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

    pub fn properties(&self) -> WindowProperties {
        let RequestResponse::WindowProps {
            size,
            loc,
            class,
            title,
            focused,
            floating,
            fullscreen_or_maximized,
        } = request(Request::GetWindowProps { window_id: self.0 })
        else {
            unreachable!()
        };

        WindowProperties {
            size,
            loc,
            class,
            title,
            focused,
            floating,
            fullscreen_or_maximized,
        }
    }

    pub fn toggle_tag(&self, tag: &TagHandle) {
        let msg = Msg::ToggleTagOnWindow {
            window_id: self.0,
            tag_id: tag.0,
        };

        send_msg(msg).unwrap();
    }

    pub fn move_to_tag(&self, tag: &TagHandle) {
        let msg = Msg::MoveWindowToTag {
            window_id: self.0,
            tag_id: tag.0,
        };

        send_msg(msg).unwrap();
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize)]
pub enum FloatingOrTiled {
    Floating,
    Tiled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FullscreenOrMaximized {
    Neither,
    Fullscreen,
    Maximized,
}
