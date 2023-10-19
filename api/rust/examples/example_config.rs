use pinnacle_api::{Modifier, MouseButton, MouseEdge};

fn main() {
    pinnacle_api::setup(|pinnacle| {
        pinnacle.process.spawn(vec!["alacritty"]).unwrap();

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
