use pinnacle_api::{Modifier, MouseButton, MouseEdge};

fn main() {
    pinnacle_api::setup(|pinnacle| {
        pinnacle.output.connect_for_all(move |output| {
            pinnacle.tag.add(&output, &["1", "2", "3", "4", "5"]);
            pinnacle.tag.get("1", Some(&output)).unwrap().toggle();
        });

        pinnacle.input.keybind(&[Modifier::Ctrl], 'a', move || {
            pinnacle.process.spawn(vec!["alacritty"]).unwrap();
        });

        pinnacle.input.mousebind(
            &[Modifier::Ctrl],
            MouseButton::Left,
            MouseEdge::Press,
            || {},
        );
    })
    .unwrap();
}
