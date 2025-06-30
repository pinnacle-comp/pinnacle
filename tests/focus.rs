use common::fixture::Fixture;
use pinnacle::{focus::keyboard::KeyboardFocusTarget, state::WithState, tag::Tag};
use pinnacle_api::layout::{generators::MasterStack, LayoutGenerator, LayoutNode};
use smithay::{output::Output, utils::Rectangle};
use test_log::test;

mod common;

fn set_up() -> (Fixture, Output, Output) {
    let mut fixture = Fixture::new();

    let output_1 = fixture.add_output(Rectangle::new((0, 0).into(), (100, 100).into()));
    output_1.with_state_mut(|state| {
        let tag = Tag::new("1".to_string());
        tag.set_active(true);
        state.add_tags([tag]);
    });
    fixture.pinnacle().focus_output(&output_1);

    let output_2 = fixture.add_output(Rectangle::new((100, 0).into(), (100, 100).into()));
    output_2.with_state_mut(|state| {
        let tag = Tag::new("1".to_string());
        tag.set_active(true);
        state.add_tags([tag]);
    });

    fixture
        .runtime_handle()
        .block_on(pinnacle_api::connect())
        .unwrap();

    (fixture, output_1, output_2)
}

#[test]
fn output_focus() {
    let (mut fixture, op1, op2) = set_up();

    assert_eq!(fixture.pinnacle().focused_output(), Some(&op1));

    let name = op2.name();
    fixture.spawn_blocking(move || pinnacle_api::output::get_by_name(&name).unwrap().focus());

    assert_eq!(fixture.pinnacle().focused_output(), Some(&op2));
}

#[test]
fn keyboard_focus() {
    let (mut fixture, _, _) = set_up();

    fixture.spawn_blocking(|| {
        pinnacle_api::layout::manage(|args| pinnacle_api::layout::LayoutResponse {
            root_node: MasterStack::default().layout(args.window_count),
            tree_id: 0,
        });
    });

    // Add a window
    let client_id = fixture.add_client();
    let window = fixture.client(client_id).create_window();
    let surface1 = window.surface();
    window.commit();
    let window = fixture.client(client_id).create_window();
    let surface2 = window.surface();
    window.commit();
    fixture.roundtrip(client_id);

    // Commit a buffer
    let window1 = fixture.client(client_id).window_for_surface(&surface1);
    window1.attach_buffer();
    window1.ack_and_commit();
    let window2 = fixture.client(client_id).window_for_surface(&surface2);
    window2.attach_buffer();
    window2.ack_and_commit();
    fixture.roundtrip(client_id);

    // Let Pinnacle do a layout
    fixture.dispatch_until(|fixture| fixture.pinnacle().layout_state.layout_trees.len() == 1);
    fixture.roundtrip(client_id);

    // Commit the layout
    let window1 = fixture.client(client_id).window_for_surface(&surface1);
    window1.ack_and_commit();
    let window2 = fixture.client(client_id).window_for_surface(&surface2);
    window2.ack_and_commit();
    fixture.roundtrip(client_id);

    let current_focus = fixture
        .pinnacle()
        .seat
        .get_keyboard()
        .unwrap()
        .current_focus();
    assert_eq!(
        current_focus,
        Some(KeyboardFocusTarget::Window(
            fixture.pinnacle().windows[1].clone()
        ))
    );

    fixture.spawn_blocking(|| {
        pinnacle_api::window::get_all()
            .next()
            .unwrap()
            .set_focused(true)
    });

    let current_focus = fixture
        .pinnacle()
        .seat
        .get_keyboard()
        .unwrap()
        .current_focus();
    assert_eq!(
        current_focus,
        Some(KeyboardFocusTarget::Window(
            fixture.pinnacle().windows[0].clone()
        ))
    );
}
