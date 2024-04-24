use pinnacle::state::State;

mod inner {
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

    pub(crate) fn start_config() {
        main()
    }
}

pub fn run_config(state: &mut State) {
    let temp_dir = tempfile::tempdir().expect("failed to setup temp dir for socket");
    let socket_dir = temp_dir.path().to_owned();
    state
        .start_wlcs_config(
            &socket_dir,
            move || {
                inner::start_config();
                drop(temp_dir);
            },
        )
        .expect("failed to start wlcs config");
}
