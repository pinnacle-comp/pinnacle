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

/// Window management.
#[derive(Clone, Copy)]
pub struct Window {
    /// Window rules.
    pub rules: WindowRules,
}

impl Window {
    /// Get all windows with the class `class`.
    pub fn get_by_class<'a>(&self, class: &'a str) -> impl Iterator<Item = WindowHandle> + 'a {
        self.get_all()
            .filter(|win| win.properties().class.as_deref() == Some(class))
    }

    /// Get the currently focused window, or `None` if there isn't one.
    pub fn get_focused(&self) -> Option<WindowHandle> {
        self.get_all()
            .find(|win| win.properties().focused.is_some_and(|focused| focused))
    }

    /// Get all windows.
    pub fn get_all(&self) -> impl Iterator<Item = WindowHandle> {
        let RequestResponse::Windows { window_ids } = request(Request::GetWindows) else {
            unreachable!()
        };

        window_ids.into_iter().map(WindowHandle)
    }

    /// Begin a window move.
    ///
    /// This will start a window move grab with the provided button on the window the pointer
    /// is currently hovering over. Once `button` is let go, the move will end.
    pub fn begin_move(&self, button: MouseButton) {
        let msg = Msg::WindowMoveGrab {
            button: button as u32,
        };

        send_msg(msg).unwrap();
    }

    /// Begin a window resize.
    ///
    /// This will start a window resize grab with the provided button on the window the
    /// pointer is currently hovering over. Once `button` is let go, the resize will end.
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

/// Whether or not a window is floating or tiled.
#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize)]
pub enum FloatingOrTiled {
    /// The window is floating.
    ///
    /// It can be freely moved around and resized and will not respond to layouts.
    Floating,
    /// The window is tiled.
    ///
    /// It cannot be resized and can only move by swapping places with other tiled windows.
    Tiled,
}

/// Whether the window is fullscreen, maximized, or neither.
///
/// These three states are mutually exclusive. Setting a window to maximized while it is fullscreen
/// will make it stop being fullscreen and vice versa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FullscreenOrMaximized {
    /// The window is not fullscreen or maximized.
    Neither,
    /// The window is fullscreen.
    ///
    /// It will be the only rendered window on screen and will fill the output it resides on.
    /// Layer surfaces will also not be rendered while a window is fullscreen.
    Fullscreen,
    /// The window is maximized.
    ///
    /// It will fill up as much space on its output as it can, respecting any layer surfaces.
    Maximized,
}
