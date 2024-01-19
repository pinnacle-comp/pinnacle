use pinnacle_api::{input::Mod, ApiModules};

#[pinnacle_api::config(modules)]
#[tokio::main]
async fn main() {
    let ApiModules {
        pinnacle,
        process,
        window,
        input,
        output,
        tag,
    } = modules;

    input.keybind([Mod::Shift], 'a', || {
        process.spawn(["alacritty"]);
    });
}
