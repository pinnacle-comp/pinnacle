use pinnacle_api::layout::{CyclingLayoutManager, MasterStackLayout};
use pinnacle_api::ApiModules;

// Pinnacle needs to perform some setup before and after your config.
// The `#[pinnacle_api::config(modules)]` attribute does so and
// will bind all the config structs to the provided identifier.
#[pinnacle_api::config(modules)]
async fn main() {
    // Deconstruct to get all the APIs.
    #[allow(unused_variables)]
    let ApiModules {
        pinnacle,
        process,
        window,
        input,
        output,
        tag,
        layout,
        render,
    } = modules;

    let _layout_requester = layout.set_manager(CyclingLayoutManager::new([
        Box::<MasterStackLayout>::default() as _,
    ]));

    // Setup all monitors with tags "1" through "5"
    output.connect_for_all(move |op| {
        let tags = tag.add(op, ["tag"]);
        tags.first().unwrap().set_active(true);
    });

    // Enable sloppy focus
    /* window.connect_signal(WindowSignal::PointerEnter(Box::new(|win| {
        win.set_focused(true);
    }))); */
}
