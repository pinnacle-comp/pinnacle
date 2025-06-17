mod inner {
    use pinnacle_api::layout::{generators::MasterStack, LayoutGenerator, LayoutResponse};

    async fn config() {
        pinnacle_api::output::for_each_output(|output| {
            pinnacle_api::tag::add(output, ["1"])
                .next()
                .unwrap()
                .set_active(true);
        });

        pinnacle_api::window::add_window_rule(|window| {
            window.set_floating(true);
        });

        let _layout_requester = pinnacle_api::layout::manage(|args| LayoutResponse {
            root_node: MasterStack::default().layout(args.window_count),
            tree_id: 0,
        });
    }

    pinnacle_api::main!(config);

    pub(crate) fn start_config() {
        main()
    }
}

pub(crate) use inner::start_config;
