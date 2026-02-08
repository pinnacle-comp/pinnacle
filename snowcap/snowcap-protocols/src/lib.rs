pub mod snowcap_decoration_v1 {
    pub mod client {
        use wayland_client;
        use wayland_client::protocol::*;
        use wayland_protocols::ext::foreign_toplevel_list::v1::client::*;
        use wayland_protocols::xdg::shell::client::*;

        pub mod __interfaces {
            use wayland_client::protocol::__interfaces::*;
            use wayland_protocols::ext::foreign_toplevel_list::v1::client::__interfaces::*;
            use wayland_protocols::xdg::shell::client::__interfaces::*;

            wayland_scanner::generate_interfaces!("./protocol/snowcap-decoration-v1.xml");
        }
        use self::__interfaces::*;

        wayland_scanner::generate_client_code!("./protocol/snowcap-decoration-v1.xml");
    }

    pub mod server {
        use wayland_protocols::ext::foreign_toplevel_list::v1::server::*;
        use wayland_protocols::xdg::shell::server::*;
        use wayland_server;
        use wayland_server::protocol::*;

        pub mod __interfaces {
            use wayland_protocols::ext::foreign_toplevel_list::v1::server::__interfaces::*;
            use wayland_protocols::xdg::shell::server::__interfaces::*;
            use wayland_server::protocol::__interfaces::*;

            wayland_scanner::generate_interfaces!("./protocol/snowcap-decoration-v1.xml");
        }
        use self::__interfaces::*;

        wayland_scanner::generate_server_code!("./protocol/snowcap-decoration-v1.xml");
    }
}
