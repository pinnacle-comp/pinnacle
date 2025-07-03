use pinnacle::{state::WithState, tag::Tag};
use pinnacle_api::layout::{LayoutGenerator as _, generators::MasterStack};
use smithay::{output::Output, utils::Rectangle};

use crate::common::fixture::Fixture;

fn set_up() -> (Fixture, Output) {
    let mut fixture = Fixture::new();

    let output = fixture.add_output(Rectangle::new((0, 0).into(), (1920, 1080).into()));
    output.with_state_mut(|state| {
        let tag = Tag::new("1".to_string());
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

    (fixture, output)
}

#[test_log::test]
fn window_floating_size_heuristic_works() {
    let (mut fixture, _) = set_up();

    let client_id = fixture.add_client();

    fixture.spawn_floating_window_with(client_id, (500, 500), |_| ());

    fixture.spawn_windows(1, client_id).remove(0);

    assert!(fixture.pinnacle().windows[0].with_state(|state| state.layout_mode.is_floating()));

    let size = fixture.pinnacle().windows[0].geometry().size;
    assert_eq!(size, (500, 500).into());
}

#[test_log::test]
fn window_spawned_without_tags_gets_tags_after_add() {
    let (mut fixture, output) = set_up();

    output.with_state_mut(|state| state.tags.clear());

    let id = fixture.add_client();

    // Add a window
    let window = fixture.client(id).create_window();
    window.commit();
    let surface = window.surface();
    fixture.roundtrip(id);

    assert!(
        fixture
            .client(id)
            .window_for_surface(&surface)
            .current_serial()
            .is_none()
    );

    assert!(
        fixture.pinnacle().unmapped_windows[0]
            .window
            .with_state(|state| state.tags.is_empty())
    );

    assert!(fixture.pinnacle().windows.is_empty());
    assert_eq!(fixture.pinnacle().unmapped_windows.len(), 1);

    fixture.spawn_blocking(|| {
        let _ = pinnacle_api::tag::add(
            &pinnacle_api::output::get_focused().unwrap(),
            ["1", "2", "3"],
        );
    });

    assert!(
        fixture
            .client(id)
            .window_for_surface(&surface)
            .current_serial()
            .is_some()
    );

    assert!(
        fixture.pinnacle().unmapped_windows[0]
            .window
            .with_state(|state| !state.tags.is_empty())
    );
}

#[test_log::test]
fn window_tags_update_after_set_geometry() {
    let (mut fixture, output1) = set_up();
    let output2 = fixture.add_output(Rectangle::new((1920, 0).into(), (1920, 1080).into()));
    output2.with_state_mut(|state| {
        let tag = Tag::new("1".to_string());
        tag.set_active(true);
        state.add_tags([tag]);
    });
    fixture.pinnacle().focus_output(&output1);

    let id = fixture.add_client();

    fixture.spawn_floating_window_with(id, (500, 500), |_| ());

    let tags = fixture.pinnacle().windows[0].with_state(|state| state.tags.clone());
    assert_eq!(tags, output1.with_state(|state| state.tags.clone()));

    fixture.spawn_blocking(|| {
        pinnacle_api::window::get_focused()
            .unwrap()
            .set_geometry(2000, None, None, None);
    });

    let tags = fixture.pinnacle().windows[0].with_state(|state| state.tags.clone());
    assert_eq!(tags, output2.with_state(|state| state.tags.clone()));
}
