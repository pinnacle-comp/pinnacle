use pinnacle::state::Pinnacle;
mod inner {
    use pinnacle_api::layout::{CyclingLayoutManager, MasterStackLayout};
    use pinnacle_api::window::rules::{WindowRule, WindowRuleCondition};
    use pinnacle_api::ApiModules;

    #[pinnacle_api::config(modules)]
    async fn main() {
        #[allow(unused_variables)]
        let ApiModules { layout, window, .. } = modules;

        window.add_window_rule(
            WindowRuleCondition::default().all(vec![]),
            WindowRule::new().floating(true),
        );

        let _layout_requester = layout.set_manager(CyclingLayoutManager::new([
            Box::<MasterStackLayout>::default() as _,
        ]));
    }

    pub(crate) fn start_config() {
        main()
    }
}

pub fn run_config(pinnacle: &mut Pinnacle) {
    let temp_dir = tempfile::tempdir().expect("failed to setup temp dir for socket");
    let socket_dir = temp_dir.path().to_owned();
    pinnacle
        .start_wlcs_config(&socket_dir, move || {
            inner::start_config();
            drop(temp_dir);
        })
        .expect("failed to start wlcs config");
}
