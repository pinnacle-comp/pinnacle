use crate::common::fixture::Fixture;
use pinnacle::{focus::keyboard::KeyboardFocusTarget, state::WithState, tag::Tag};
use pinnacle_api::layout::{LayoutGenerator, generators::MasterStack};
use smithay::{output::Output, utils::Rectangle};
use test_log::test;

fn set_up() -> (Fixture, Output, Output) {
    let mut fixture = Fixture::new();

    let output_1 = fixture.add_output(Rectangle::new((0, 0).into(), (100, 100).into()));
    output_1.with_state_mut(|state| {
        let tag = Tag::new("1".to_string());
        tag.set_active(true);
        state.add_tags([tag]);
    });

    let output_2 = fixture.add_output(Rectangle::new((100, 0).into(), (100, 100).into()));
    output_2.with_state_mut(|state| {
        let tag = Tag::new("1".to_string());
        tag.set_active(true);
        state.add_tags([tag]);
    });

    fixture.pinnacle().focus_output(&output_1);

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

    fixture.spawn_windows(2, client_id);

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
            .try_set_focused(true)
            .unwrap()
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
