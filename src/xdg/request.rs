use smithay::{
    input::{pointer::Focus, Seat},
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel,
    utils::Rectangle,
    wayland::shell::xdg::ToplevelSurface,
};

use crate::{
    backend::Backend,
    grab::{move_grab::MoveSurfaceGrab, resize_grab::ResizeSurfaceGrab},
    State,
};

pub fn move_request<B: Backend>(
    state: &mut State<B>,
    surface: &ToplevelSurface,
    seat: &Seat<State<B>>,
    serial: smithay::utils::Serial,
) {
    println!("move_request started");

    let wl_surface = surface.wl_surface();

    let pointer = seat.get_pointer().unwrap();
    if let Some(start_data) = crate::pointer::pointer_grab_start_data(&pointer, wl_surface, serial)
    {
        let window = state
            .space
            .elements()
            .find(|w| w.toplevel().wl_surface() == wl_surface)
            .unwrap()
            .clone();

        let initial_window_loc = state.space.element_location(&window).unwrap();

        let grab = MoveSurfaceGrab {
            start_data,
            window,
            initial_window_loc,
        };

        pointer.set_grab(state, grab, serial, Focus::Clear);
    } else {
        println!("no grab start data");
    }
}

// TODO: see how this interacts with drag and drop and other grabs
pub fn move_request_force<B: Backend>(
    state: &mut State<B>,
    surface: &ToplevelSurface,
    seat: &Seat<State<B>>,
    serial: smithay::utils::Serial,
) {
    println!("move_request_force started");

    let wl_surface = surface.wl_surface();

    let pointer = seat.get_pointer().unwrap();
    let window = state
        .space
        .elements()
        .find(|w| w.toplevel().wl_surface() == wl_surface)
        .unwrap()
        .clone();

    let initial_window_loc = state.space.element_location(&window).unwrap();

    let start_data = smithay::input::pointer::GrabStartData {
        focus: pointer
            .current_focus()
            .map(|focus| (focus, initial_window_loc)),
        button: 0x110,
        location: pointer.current_location(),
    };

    let grab = MoveSurfaceGrab {
        start_data,
        window,
        initial_window_loc,
    };

    pointer.set_grab(state, grab, serial, Focus::Clear);
}

pub fn resize_request<B: Backend>(
    state: &mut State<B>,
    surface: &ToplevelSurface,
    seat: &Seat<State<B>>,
    serial: smithay::utils::Serial,
    edges: xdg_toplevel::ResizeEdge,
    button_used: u32,
) {
    let wl_surface = surface.wl_surface();

    let pointer = seat.get_pointer().unwrap();

    if let Some(start_data) = crate::pointer::pointer_grab_start_data(&pointer, wl_surface, serial)
    {
        let window = state
            .space
            .elements()
            .find(|w| w.toplevel().wl_surface() == wl_surface)
            .unwrap()
            .clone(); // TODO: move this search into its own function

        let initial_window_loc = state.space.element_location(&window).unwrap();
        let initial_window_size = window.geometry().size;

        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Resizing);
        });

        surface.send_pending_configure();

        let grab = ResizeSurfaceGrab::start(
            start_data,
            window,
            edges,
            Rectangle::from_loc_and_size(initial_window_loc, initial_window_size),
            button_used,
        );

        pointer.set_grab(state, grab, serial, Focus::Clear);
    }
}

pub fn resize_request_force<B: Backend>(
    state: &mut State<B>,
    surface: &ToplevelSurface,
    seat: &Seat<State<B>>,
    serial: smithay::utils::Serial,
    edges: xdg_toplevel::ResizeEdge,
    button_used: u32,
) {
    println!("resize_request_force started with edges {:?}", edges);
    let wl_surface = surface.wl_surface();

    let pointer = seat.get_pointer().unwrap();

    let window = state
        .space
        .elements()
        .find(|w| w.toplevel().wl_surface() == wl_surface)
        .unwrap()
        .clone(); // TODO: move this search into its own function

    let initial_window_loc = state.space.element_location(&window).unwrap();
    let initial_window_size = window.geometry().size;

    surface.with_pending_state(|state| {
        println!("setting xdg state to Resizing");
        state.states.set(xdg_toplevel::State::Resizing);
    });

    surface.send_pending_configure();

    let start_data = smithay::input::pointer::GrabStartData {
        focus: pointer
            .current_focus()
            .map(|focus| (focus, initial_window_loc)),
        button: button_used,
        location: pointer.current_location(),
    };

    let grab = ResizeSurfaceGrab::start(
        start_data,
        window,
        edges,
        Rectangle::from_loc_and_size(initial_window_loc, initial_window_size),
        button_used,
    );

    pointer.set_grab(state, grab, serial, Focus::Clear);
}
