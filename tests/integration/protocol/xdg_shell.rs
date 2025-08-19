//! xdg_shell test suite
//!
//! This suite ensure the correctness of the implementation.
//!
//! Delegated methods are assumed to be correct, and should not be tested. If an error is found in
//! these, it should be tested and fixed upstream.
//!
//! Some methods are only implemented for their side-effect on other protocols, and should not be
//! tested here either. Instead, testing for the side-effect should be done in a test-suite for
//! that protocol. For example, `app_id_changed` forward the app_id to foreign_toplevel. Since this
//! protocol should eventually be covered by tests, there's no point in checking it here.
//!
//! ## To-Do:
//! - Add tests for move & resize. These need an instrumented pointer since they must be triggered by a
//! click.
//! - Add tests for set_minimize. Not implemented on the server yet.
//! - Add tests for unmapped functions
//!
//! Resources:
//! - https://wayland.app/protocols/xdg-shell
//! - https://gitlab.freedesktop.org/wayland/wayland-protocols/-/tree/main/stable/xdg-shell?ref_type=heads
//!

use crate::common::fixture::Fixture;
use pinnacle::{state::WithState, tag::Tag};
use pinnacle_api::layout::{LayoutGenerator as _, generators::MasterStack};

use smithay::{output::Output, utils::Rectangle};

fn set_up() -> (Fixture, Output, Output) {
    let mut fixture = Fixture::new();

    let output = fixture.add_output(Rectangle::new((0, 0).into(), (1920, 1080).into()));
    output.with_state_mut(|state| {
        let tag = Tag::new("1".to_string());
        tag.set_active(true);
        state.add_tags([tag]);
    });

    let output2 = fixture.add_output(Rectangle::new((1920, 1080).into(), (1920, 1080).into()));
    output2.with_state_mut(|state| {
        let tag = Tag::new("2".to_string());
        tag.set_active(true);
        state.add_tags([tag]);
    });

    fixture.pinnacle().focus_output(&output);

    fixture
        .runtime_handle()
        .block_on(pinnacle_api::connect())
        .unwrap();

    fixture.spawn_blocking(|| {
        pinnacle_api::layout::manage(|args| pinnacle_api::layout::LayoutResponse {
            root_node: MasterStack::default().layout(args.window_count),
            tree_id: 0,
        });
    });

    (fixture, output, output2)
}

#[test_log::test]
fn mapped_fullscreen() {
    let (mut fixture, output, _) = set_up();

    let client_id = fixture.add_client();

    let surfaces = fixture.spawn_windows(1, client_id);
    let surface = &surfaces[0];

    fixture
        .client(client_id)
        .window_for_surface(surface)
        .set_fullscreen(None);
    fixture.roundtrip(client_id);

    fixture.wait_client_configure(client_id);
    fixture.flush();

    let tags = fixture.pinnacle().windows[0].with_state(|state| state.tags.clone());

    assert!(
        fixture
            .client(client_id)
            .window_for_surface(surface)
            .fullscreen
    );
    assert_eq!(output.with_state(|state| state.tags.clone()), tags);

    fixture
        .client(client_id)
        .window_for_surface(surface)
        .unset_fullscreen();
    fixture.roundtrip(client_id);
    fixture.wait_client_configure(client_id);
    fixture.flush();

    assert!(
        !fixture
            .client(client_id)
            .window_for_surface(surface)
            .fullscreen
    );
}

#[test_log::test]
fn unmapped_fullscreen() {
    let (mut fixture, _, _) = set_up();

    let client_id = fixture.add_client();

    // Use floating window since the layout tree will not update
    let surface =
        fixture.spawn_floating_window_with(client_id, (500, 500), |w| w.set_fullscreen(None));

    assert!(
        fixture
            .client(client_id)
            .window_for_surface(&surface)
            .fullscreen
    );
}

#[test_log::test]
fn mapped_fullscreen_twice() {
    let (mut fixture, output, _) = set_up();

    let client_id = fixture.add_client();

    let surfaces = fixture.spawn_windows(1, client_id);
    let surface = &surfaces[0];

    fixture
        .client(client_id)
        .window_for_surface(surface)
        .set_fullscreen(None);
    fixture.roundtrip(client_id);
    fixture.wait_client_configure(client_id);
    fixture.flush();

    let tags = fixture.pinnacle().windows[0].with_state(|state| state.tags.clone());

    assert!(
        fixture
            .client(client_id)
            .window_for_surface(surface)
            .fullscreen
    );
    assert_eq!(output.with_state(|state| state.tags.clone()), tags);

    fixture
        .client(client_id)
        .window_for_surface(surface)
        .set_fullscreen(None);
    fixture.roundtrip(client_id);
    fixture.wait_client_configure(client_id);
    fixture.flush();
}

#[test_log::test]
fn mapped_fullscreen_twice_two_outputs() {
    let (mut fixture, output, output2) = set_up();

    let client_id = fixture.add_client();

    let surfaces = fixture.spawn_windows(1, client_id);
    let surface = &surfaces[0];

    let outputs = fixture.client(client_id).wl_outputs().clone();

    fixture
        .client(client_id)
        .window_for_surface(surface)
        .set_fullscreen(outputs.iter().nth(0));
    fixture.roundtrip(client_id);
    fixture.wait_client_configure(client_id);
    fixture.flush();

    let tags = fixture.pinnacle().windows[0].with_state(|state| state.tags.clone());

    assert!(
        fixture
            .client(client_id)
            .window_for_surface(surface)
            .fullscreen
    );
    assert_eq!(output.with_state(|state| state.tags.clone()), tags);

    fixture
        .client(client_id)
        .window_for_surface(surface)
        .set_fullscreen(outputs.iter().nth(1));
    fixture.roundtrip(client_id);
    fixture.wait_client_configure(client_id);
    fixture.flush();

    let window = fixture.pinnacle().windows[0].clone();
    let output_geo = fixture.pinnacle().space.output_geometry(&output2);
    let window_geo = fixture.pinnacle().space.element_geometry(&window);

    assert_eq!(output_geo, window_geo);
}

#[test_log::test]
fn mapped_set_fullscreen_on_output() {
    let (mut fixture, _, output) = set_up();

    let client_id = fixture.add_client();

    let surfaces = fixture.spawn_windows(1, client_id);
    let surface = &surfaces[0];

    let outputs = fixture.client(client_id).wl_outputs().clone();

    fixture
        .client(client_id)
        .window_for_surface(surface)
        .set_fullscreen(outputs.iter().nth(1));
    fixture.roundtrip(client_id);
    fixture.wait_client_configure(client_id);
    fixture.flush();

    let tags = fixture.pinnacle().windows[0].with_state(|state| state.tags.clone());

    assert!(
        fixture
            .client(client_id)
            .window_for_surface(surface)
            .fullscreen
    );
    assert_eq!(output.with_state(|state| state.tags.clone()), tags);
}

#[test_log::test]
fn mapped_set_fullscreen_on_output_update_floating_loc() {
    let (mut fixture, _, output) = set_up();

    let client_id = fixture.add_client();

    let surface = fixture.spawn_floating_window_with(client_id, (500, 500), |_| ());

    let outputs = fixture.client(client_id).wl_outputs().clone();

    fixture
        .client(client_id)
        .window_for_surface(&surface)
        .set_fullscreen(outputs.iter().nth(1));
    fixture.roundtrip(client_id);
    fixture.wait_client_configure(client_id);
    fixture.flush();

    let tags = fixture.pinnacle().windows[0].with_state(|state| state.tags.clone());

    assert!(
        fixture
            .client(client_id)
            .window_for_surface(&surface)
            .fullscreen
    );
    assert_eq!(output.with_state(|state| state.tags.clone()), tags);

    fixture
        .client(client_id)
        .window_for_surface(&surface)
        .unset_fullscreen();
    fixture.roundtrip(client_id);
    fixture.wait_client_configure(client_id);
    fixture.flush();

    let window = fixture.pinnacle().windows[0].clone();
    let window_geo = fixture.pinnacle().space.element_geometry(&window).unwrap();
    let output_geo = fixture.pinnacle().space.output_geometry(&output).unwrap();

    assert!(output_geo.intersection(window_geo).is_some());
}

#[test_log::test]
fn unmapped_set_fullscreen_on_output() {
    let (mut fixture, _, output) = set_up();

    let client_id = fixture.add_client();

    let outputs = fixture.client(client_id).wl_outputs().clone();

    // Use floating window since the layout tree will not update
    let surface = fixture.spawn_floating_window_with(client_id, (500, 500), |w| {
        w.set_fullscreen(outputs.iter().nth(1))
    });

    let tags = fixture.pinnacle().windows[0].with_state(|state| state.tags.clone());

    assert!(
        fixture
            .client(client_id)
            .window_for_surface(&surface)
            .fullscreen
    );
    assert_eq!(output.with_state(|state| state.tags.clone()), tags);
}

#[test_log::test]
fn mapped_set_maximized() {
    let (mut fixture, output, _) = set_up();

    let client_id = fixture.add_client();

    let surfaces = fixture.spawn_windows(1, client_id);
    let surface = &surfaces[0];

    fixture
        .client(client_id)
        .window_for_surface(surface)
        .set_maximized();
    fixture.roundtrip(client_id);
    fixture.wait_client_configure(client_id);
    fixture.flush();

    let tags = fixture.pinnacle().windows[0].with_state(|state| state.tags.clone());

    assert!(
        fixture
            .client(client_id)
            .window_for_surface(surface)
            .maximized
    );
    assert_eq!(output.with_state(|state| state.tags.clone()), tags);

    fixture
        .client(client_id)
        .window_for_surface(surface)
        .unset_maximized();
    fixture.roundtrip(client_id);
    fixture.wait_client_configure(client_id);
    fixture.flush();

    assert!(
        !fixture
            .client(client_id)
            .window_for_surface(surface)
            .maximized
    );
}

#[test_log::test]
fn unmapped_set_maximize() {
    let (mut fixture, _, _) = set_up();

    let client_id = fixture.add_client();

    // Use floating window since the layout tree will not update
    let surface = fixture.spawn_floating_window_with(client_id, (500, 500), |w| w.set_maximized());

    assert!(
        fixture
            .client(client_id)
            .window_for_surface(&surface)
            .maximized
    );
}

#[test_log::test]
fn mapped_set_maximized_twice() {
    let (mut fixture, output, _) = set_up();

    let client_id = fixture.add_client();

    let surfaces = fixture.spawn_windows(1, client_id);
    let surface = &surfaces[0];

    fixture
        .client(client_id)
        .window_for_surface(surface)
        .set_maximized();
    fixture.roundtrip(client_id);
    fixture.wait_client_configure(client_id);
    fixture.flush();

    let tags = fixture.pinnacle().windows[0].with_state(|state| state.tags.clone());

    assert!(
        fixture
            .client(client_id)
            .window_for_surface(surface)
            .maximized
    );
    assert_eq!(output.with_state(|state| state.tags.clone()), tags);

    // If the surface was already maximized, the compositor will still emit a configure event with
    // the "maximized" state.
    fixture
        .client(client_id)
        .window_for_surface(surface)
        .set_maximized();
    fixture.roundtrip(client_id);
    fixture.wait_client_configure(client_id);
    fixture.flush();
}

#[test_log::test]
fn mapped_set_maximized_after_fullscreen() {
    let (mut fixture, _, _) = set_up();

    let client_id = fixture.add_client();

    let surfaces = fixture.spawn_windows(1, client_id);
    let surface = &surfaces[0];

    fixture
        .client(client_id)
        .window_for_surface(surface)
        .set_fullscreen(None);
    fixture.roundtrip(client_id);
    fixture.wait_client_configure(client_id);
    fixture.flush();

    assert!(
        fixture
            .client(client_id)
            .window_for_surface(surface)
            .fullscreen
    );

    fixture
        .client(client_id)
        .window_for_surface(surface)
        .set_maximized();
    fixture.roundtrip(client_id);
    fixture.wait_client_configure(client_id);
    fixture.flush();

    // If the surface is in a fullscreen state, this request has no direct effect.
    assert!(
        fixture
            .client(client_id)
            .window_for_surface(surface)
            .fullscreen
    );
    assert!(
        !fixture
            .client(client_id)
            .window_for_surface(surface)
            .maximized
    );
}
