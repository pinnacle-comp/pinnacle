use std::os::unix::net::UnixStream;

use smithay::utils::{Logical, Point};

pub enum WlcsEvent {
    /// Stop the running server
    Exit,
    /// Create a new client from given RawFd
    NewClient {
        stream: UnixStream,
        client_id: i32,
    },
    /// Position this window from the client associated with this Fd on the global space
    PositionWindow {
        client_id: i32,
        surface_id: u32,
        location: Point<i32, Logical>,
    },
    /* Pointer related events */
    /// A new pointer device is available
    NewPointer {
        device_id: u32,
    },
    /// Move the pointer in absolute coordinate space
    PointerMoveAbsolute {
        device_id: u32,
        location: Point<f64, Logical>,
    },
    /// Move the pointer in relative coordinate space
    PointerMoveRelative {
        device_id: u32,
        delta: Point<f64, Logical>,
    },
    /// Press a pointer button
    PointerButtonDown {
        device_id: u32,
        button_id: i32,
    },
    /// Release a pointer button
    PointerButtonUp {
        device_id: u32,
        button_id: i32,
    },
    /// A pointer device is removed
    PointerRemoved {
        device_id: u32,
    },
    /* Touch related events */
    /// A new touch device is available
    NewTouch {
        device_id: u32,
    },
    /// A touch point is down
    TouchDown {
        device_id: u32,
        location: Point<f64, Logical>,
    },
    /// A touch point moved
    TouchMove {
        device_id: u32,
        location: Point<f64, Logical>,
    },
    /// A touch point is up
    TouchUp {
        device_id: u32,
    },
    TouchRemoved {
        device_id: u32,
    },
}
