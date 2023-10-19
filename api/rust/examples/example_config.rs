fn main() {
    pinnacle_api::setup(|pinnacle| {
        pinnacle.process.spawn(vec!["alacritty"]).unwrap();
    })
    .unwrap();
}
