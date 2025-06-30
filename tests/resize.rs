use common::fixture::Fixture;
use pinnacle::{state::WithState, tag::Tag};
use pinnacle_api::layout::LayoutNode;
use smithay::utils::Rectangle;

mod common;

fn set_up() -> Fixture {
    let mut fixture = Fixture::new();

    let output = fixture.add_output(Rectangle::new((0, 0).into(), (1920, 1080).into()));
    output.with_state_mut(|state| {
        let tag = Tag::new("1".to_string());
        tag.set_active(true);
        state.add_tags([tag]);
    });

    fixture
        .runtime_handle()
        .block_on(pinnacle_api::connect())
        .unwrap();

    fixture
}

#[test_log::test]
fn test_thing() {
    let mut fixture = set_up();
    let handle = fixture.runtime_handle();
    let _guard = handle.enter();

    pinnacle_api::layout::manage(|_| pinnacle_api::layout::LayoutResponse {
        root_node: LayoutNode::new(),
        tree_id: 0,
    });

    // Add a window
    let client_id = fixture.add_client();
    let window = fixture.client(client_id).create_window();
    let surface = window.surface();
    window.commit();
    fixture.roundtrip(client_id);

    // Commit a buffer
    let window = fixture.client(client_id).window_for_surface(&surface);
    window.attach_buffer();
    window.ack_and_commit();
    fixture.roundtrip(client_id);

    // Let Pinnacle do a layout
    fixture.dispatch_until(|fixture| fixture.pinnacle().layout_state.layout_trees.len() == 1);
    fixture.roundtrip(client_id);

    // Commit the layout
    let window = fixture.client(client_id).window_for_surface(&surface);
    window.ack_and_commit();
    fixture.roundtrip(client_id);

    let win = fixture.pinnacle().windows.first().unwrap();

    assert_eq!(
        win.geometry(),
        Rectangle::new((0, 0).into(), (1920, 1080).into())
    );
}
