use pinnacle_api::layout::{CyclingLayoutManager, MasterStackLayout};
use pinnacle_api::ApiModules;

#[pinnacle_api::config(modules)]
async fn main() {
    #[allow(unused_variables)]
    let ApiModules { layout, .. } = modules;

    let _layout_requester = layout.set_manager(CyclingLayoutManager::new([
        Box::<MasterStackLayout>::default() as _,
    ]));
}
