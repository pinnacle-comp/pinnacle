mod common;

use std::time::Duration;

use common::{
    grpc::{pinnacle, tag, window},
    rust::run_rust,
};
use pinnacle_api_defs::pinnacle::v1::{
    BackendRequest, QuitRequest, ReloadConfigRequest, SetXwaylandClientSelfScaleRequest,
};
use test_log::test;
use tokio::sync::Mutex;

static MUTEX: Mutex<()> = Mutex::const_new(());

#[tokio::main]
#[self::test]
async fn pinnacle_v1() -> anyhow::Result<()> {
    let _guard = MUTEX.lock().await;

    let temp_file = tempfile::tempdir()?;

    let (pinnacle_v1, mut recv) = pinnacle::v1::PinnacleService::new();
    pinnacle_v1.quit([QuitRequest {}]);
    pinnacle_v1.reload_config([ReloadConfigRequest {}]);
    pinnacle_v1.backend([BackendRequest {}]);
    pinnacle_v1
        .set_xwayland_client_self_scale([SetXwaylandClientSelfScaleRequest { self_scale: true }]);

    let pinnacle_v1_server =
        pinnacle_api_defs::pinnacle::v1::pinnacle_service_server::PinnacleServiceServer::new(
            pinnacle_v1.clone(),
        );

    let _grpc_server_join =
        start_test_grpc_server!(temp_file.path().join("grpc.sock"), pinnacle_v1_server);

    let _ = run_rust(|| {
        catch!(pinnacle_api::pinnacle::quit());
        catch!(pinnacle_api::pinnacle::reload_config());
        catch!(pinnacle_api::pinnacle::backend());
        catch!(pinnacle_api::pinnacle::set_xwayland_self_scaling(true));
    });

    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(1)) => (),
        err = recv.recv() => {
            anyhow::bail!(err.unwrap());
        }
    };

    pinnacle_v1.is_finished()
}

#[tokio::main]
#[self::test]
async fn tag_v1() -> anyhow::Result<()> {
    use pinnacle_api_defs::pinnacle::tag::v1;
    use pinnacle_api_defs::pinnacle::util::v1 as util;

    let _guard = MUTEX.lock().await;

    let temp_file = tempfile::tempdir()?;

    let (tag_v1, mut recv) = tag::v1::TagService::new();
    tag_v1.get([v1::GetRequest {}]);
    tag_v1.get_active([v1::GetActiveRequest { tag_id: 5 }]);
    tag_v1.get_name([v1::GetNameRequest { tag_id: 5 }]);
    tag_v1.get_output_name([v1::GetOutputNameRequest { tag_id: 5 }]);
    tag_v1.set_active([
        v1::SetActiveRequest {
            tag_id: 5,
            set_or_toggle: util::SetOrToggle::Set.into(),
        },
        v1::SetActiveRequest {
            tag_id: 5,
            set_or_toggle: util::SetOrToggle::Unset.into(),
        },
        v1::SetActiveRequest {
            tag_id: 5,
            set_or_toggle: util::SetOrToggle::Toggle.into(),
        },
    ]);
    tag_v1.switch_to([v1::SwitchToRequest { tag_id: 5 }]);
    tag_v1.add([v1::AddRequest {
        output_name: "mogus".into(),
        tag_names: vec!["1".into(), "2".into(), "3".into()],
    }]);
    tag_v1.remove([
        v1::RemoveRequest { tag_ids: vec![5] },
        v1::RemoveRequest {
            tag_ids: vec![0, 1, 2],
        },
    ]);

    let tag_v1_server =
        pinnacle_api_defs::pinnacle::tag::v1::tag_service_server::TagServiceServer::new(
            tag_v1.clone(),
        );

    let _grpc_server_join =
        start_test_grpc_server!(temp_file.path().join("grpc.sock"), tag_v1_server);

    let _ = run_rust(|| {
        catch!(pinnacle_api::tag::get_all());

        let tag = pinnacle_api::tag::TagHandle::from_id(5);
        catch!(tag.active());
        catch!(tag.name());
        catch!(tag.output());
        catch!(tag.set_active(true));
        catch!(tag.set_active(false));
        catch!(tag.toggle_active());
        catch!(tag.switch_to());

        let output = pinnacle_api::output::OutputHandle::from_name("mogus");
        catch!(pinnacle_api::tag::add(&output, ["1", "2", "3"]));

        catch!(tag.remove());
        catch!(pinnacle_api::tag::remove([
            pinnacle_api::tag::TagHandle::from_id(0),
            pinnacle_api::tag::TagHandle::from_id(1),
            pinnacle_api::tag::TagHandle::from_id(2),
        ]));
    });

    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(1)) => (),
        err = recv.recv() => {
            anyhow::bail!(err.unwrap());
        }
    };

    tag_v1.is_finished()
}

#[tokio::main]
#[self::test]
async fn output_v1() -> anyhow::Result<()> {
    use pinnacle_api_defs::pinnacle::output::v1;
    use pinnacle_api_defs::pinnacle::util::v1 as util;

    let _guard = MUTEX.lock().await;

    let temp_file = tempfile::tempdir()?;

    let (output_v1, mut recv) = crate::common::grpc::output::v1::OutputService::new();
    output_v1.get([v1::GetRequest {}]);
    output_v1.set_loc([v1::SetLocRequest {
        output_name: "photochad".into(),
        x: 22,
        y: 33,
    }]);
    output_v1.set_mode([
        v1::SetModeRequest {
            output_name: "photochad".into(),
            size: Some(util::Size {
                width: 500,
                height: 128,
            }),
            refresh_rate_mhz: Some(7777),
            custom: false,
        },
        v1::SetModeRequest {
            output_name: "photochad".into(),
            size: Some(util::Size {
                width: 500,
                height: 128,
            }),
            refresh_rate_mhz: None,
            custom: false,
        },
        v1::SetModeRequest {
            output_name: "photochad".into(),
            size: Some(util::Size {
                width: 500,
                height: 128,
            }),
            refresh_rate_mhz: Some(7777),
            custom: true,
        },
    ]);
    output_v1.set_modeline([v1::SetModelineRequest {
        output_name: "photochad".into(),
        modeline: Some(v1::Modeline {
            clock: 173.0,
            hdisplay: 1920,
            hsync_start: 2048,
            hsync_end: 2248,
            htotal: 2576,
            vdisplay: 1080,
            vsync_start: 1083,
            vsync_end: 1088,
            vtotal: 1120,
            hsync: false,
            vsync: true,
        }),
    }]);
    output_v1.set_scale([
        v1::SetScaleRequest {
            output_name: "photochad".into(),
            scale: 1.5,
            abs_or_rel: util::AbsOrRel::Absolute.into(),
        },
        v1::SetScaleRequest {
            output_name: "photochad".into(),
            scale: 0.25,
            abs_or_rel: util::AbsOrRel::Relative.into(),
        },
    ]);
    output_v1.set_transform([v1::SetTransformRequest {
        output_name: "photochad".into(),
        transform: v1::Transform::Flipped270.into(),
    }]);
    output_v1.set_powered([
        v1::SetPoweredRequest {
            output_name: "photochad".into(),
            set_or_toggle: util::SetOrToggle::Set.into(),
        },
        v1::SetPoweredRequest {
            output_name: "photochad".into(),
            set_or_toggle: util::SetOrToggle::Toggle.into(),
        },
    ]);
    output_v1.get_info([v1::GetInfoRequest {
        output_name: "photochad".into(),
    }]);
    output_v1.get_loc([v1::GetLocRequest {
        output_name: "photochad".into(),
    }]);
    output_v1.get_logical_size([v1::GetLogicalSizeRequest {
        output_name: "photochad".into(),
    }]);
    output_v1.get_physical_size([v1::GetPhysicalSizeRequest {
        output_name: "photochad".into(),
    }]);
    output_v1.get_modes([v1::GetModesRequest {
        output_name: "photochad".into(),
    }]);
    output_v1.get_focused([v1::GetFocusedRequest {
        output_name: "photochad".into(),
    }]);
    output_v1.get_tag_ids([v1::GetTagIdsRequest {
        output_name: "photochad".into(),
    }]);
    output_v1.get_scale([v1::GetScaleRequest {
        output_name: "photochad".into(),
    }]);
    output_v1.get_transform([v1::GetTransformRequest {
        output_name: "photochad".into(),
    }]);
    output_v1.get_enabled([v1::GetEnabledRequest {
        output_name: "photochad".into(),
    }]);
    output_v1.get_powered([v1::GetPoweredRequest {
        output_name: "photochad".into(),
    }]);
    output_v1.get_focus_stack_window_ids([v1::GetFocusStackWindowIdsRequest {
        output_name: "photochad".into(),
    }]);

    let output_v1_server =
        pinnacle_api_defs::pinnacle::output::v1::output_service_server::OutputServiceServer::new(
            output_v1.clone(),
        );

    let _grpc_server_join =
        start_test_grpc_server!(temp_file.path().join("grpc.sock"), output_v1_server);

    let _ = run_rust(|| {
        catch!(pinnacle_api::output::get_all());

        let output = pinnacle_api::output::OutputHandle::from_name("photochad");
        catch!(output.set_loc(22, 33));
        catch!(output.set_mode(500, 128, 7777));
        catch!(output.set_mode(500, 128, None));
        catch!(output.set_custom_mode(500, 128, 7777));
        catch!(output.set_modeline(
            "173.00 1920 2048 2248 2576 1080 1083 1088 1120 -hsync +vsync"
                .parse()
                .unwrap(),
        ));
        catch!(output.set_scale(1.5));
        catch!(output.change_scale(0.25));
        catch!(output.set_transform(pinnacle_api::output::Transform::Flipped270));
        catch!(output.set_powered(true));
        catch!(output.toggle_powered());
        catch!(output.make());
        catch!(output.loc());
        catch!(output.logical_size());
        catch!(output.physical_size());
        catch!(output.modes());
        catch!(output.focused());
        catch!(output.tags());
        catch!(output.scale());
        catch!(output.transform());
        catch!(output.enabled());
        catch!(output.powered());
        catch!(output.keyboard_focus_stack());
    });

    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(1)) => (),
        err = recv.recv() => {
            anyhow::bail!(err.unwrap());
        }
    };

    output_v1.is_finished()
}

#[tokio::main]
#[self::test]
async fn window_v1() -> anyhow::Result<()> {
    use pinnacle_api_defs::pinnacle::util::v1 as util;
    use pinnacle_api_defs::pinnacle::window::v1;

    let _guard = MUTEX.lock().await;

    let temp_file = tempfile::tempdir()?;

    let (window_v1, mut recv) = window::v1::WindowService::new();
    window_v1.get([v1::GetRequest {}]);
    window_v1.get_app_id([v1::GetAppIdRequest { window_id: 5 }]);
    window_v1.get_title([v1::GetTitleRequest { window_id: 5 }]);
    window_v1.get_loc([v1::GetLocRequest { window_id: 5 }]);
    window_v1.get_size([v1::GetSizeRequest { window_id: 5 }]);
    window_v1.get_focused([v1::GetFocusedRequest { window_id: 5 }]);
    window_v1.get_layout_mode([v1::GetLayoutModeRequest { window_id: 5 }]);
    window_v1.get_tag_ids([v1::GetTagIdsRequest { window_id: 5 }]);
    window_v1.close([v1::CloseRequest { window_id: 5 }]);
    window_v1.set_geometry([
        v1::SetGeometryRequest {
            window_id: 5,
            x: Some(500),
            y: None,
            w: None,
            h: None,
        },
        v1::SetGeometryRequest {
            window_id: 5,
            x: None,
            y: None,
            w: Some(640),
            h: Some(480),
        },
    ]);
    window_v1.set_fullscreen([
        v1::SetFullscreenRequest {
            window_id: 5,
            set_or_toggle: util::SetOrToggle::Set.into(),
        },
        v1::SetFullscreenRequest {
            window_id: 5,
            set_or_toggle: util::SetOrToggle::Unset.into(),
        },
        v1::SetFullscreenRequest {
            window_id: 5,
            set_or_toggle: util::SetOrToggle::Toggle.into(),
        },
    ]);
    window_v1.set_maximized([
        v1::SetMaximizedRequest {
            window_id: 5,
            set_or_toggle: util::SetOrToggle::Set.into(),
        },
        v1::SetMaximizedRequest {
            window_id: 5,
            set_or_toggle: util::SetOrToggle::Unset.into(),
        },
        v1::SetMaximizedRequest {
            window_id: 5,
            set_or_toggle: util::SetOrToggle::Toggle.into(),
        },
    ]);
    window_v1.set_floating([
        v1::SetFloatingRequest {
            window_id: 5,
            set_or_toggle: util::SetOrToggle::Set.into(),
        },
        v1::SetFloatingRequest {
            window_id: 5,
            set_or_toggle: util::SetOrToggle::Unset.into(),
        },
        v1::SetFloatingRequest {
            window_id: 5,
            set_or_toggle: util::SetOrToggle::Toggle.into(),
        },
    ]);
    window_v1.set_focused([
        v1::SetFocusedRequest {
            window_id: 5,
            set_or_toggle: util::SetOrToggle::Set.into(),
        },
        v1::SetFocusedRequest {
            window_id: 5,
            set_or_toggle: util::SetOrToggle::Unset.into(),
        },
        v1::SetFocusedRequest {
            window_id: 5,
            set_or_toggle: util::SetOrToggle::Toggle.into(),
        },
    ]);
    window_v1.set_decoration_mode([
        v1::SetDecorationModeRequest {
            window_id: 5,
            decoration_mode: v1::DecorationMode::ClientSide.into(),
        },
        v1::SetDecorationModeRequest {
            window_id: 5,
            decoration_mode: v1::DecorationMode::ServerSide.into(),
        },
    ]);
    window_v1.move_to_tag([v1::MoveToTagRequest {
        window_id: 5,
        tag_id: 1,
    }]);
    window_v1.set_tag([
        v1::SetTagRequest {
            window_id: 5,
            tag_id: 1,
            set_or_toggle: util::SetOrToggle::Set.into(),
        },
        v1::SetTagRequest {
            window_id: 5,
            tag_id: 1,
            set_or_toggle: util::SetOrToggle::Unset.into(),
        },
        v1::SetTagRequest {
            window_id: 5,
            tag_id: 1,
            set_or_toggle: util::SetOrToggle::Toggle.into(),
        },
    ]);
    window_v1.raise([v1::RaiseRequest { window_id: 5 }]);
    const LMB: u32 = 272;
    window_v1.move_grab([v1::MoveGrabRequest { button: LMB }]);
    window_v1.resize_grab([v1::ResizeGrabRequest { button: LMB }]);

    let window_v1_server =
        pinnacle_api_defs::pinnacle::window::v1::window_service_server::WindowServiceServer::new(
            window_v1.clone(),
        );

    let _grpc_server_join =
        start_test_grpc_server!(temp_file.path().join("grpc.sock"), window_v1_server);

    let _ = run_rust(|| {
        use pinnacle_api::window;

        let win = window::WindowHandle::from_id(5);
        let tag = pinnacle_api::tag::TagHandle::from_id(1);

        catch!(window::get_all());
        catch!(win.app_id());
        catch!(win.title());
        catch!(win.loc());
        catch!(win.size());
        catch!(win.focused());
        catch!(win.layout_mode());
        catch!(win.tags());
        catch!(win.close());
        catch!(win.set_geometry(500, None, None, None));
        catch!(win.set_geometry(None, None, 640, 480));
        catch!(win.set_fullscreen(true));
        catch!(win.set_fullscreen(false));
        catch!(win.toggle_fullscreen());
        catch!(win.set_maximized(true));
        catch!(win.set_maximized(false));
        catch!(win.toggle_maximized());
        catch!(win.set_floating(true));
        catch!(win.set_floating(false));
        catch!(win.toggle_floating());
        catch!(win.set_focused(true));
        catch!(win.set_focused(false));
        catch!(win.toggle_focused());
        catch!(win.set_decoration_mode(window::DecorationMode::ClientSide));
        catch!(win.set_decoration_mode(window::DecorationMode::ServerSide));
        catch!(win.move_to_tag(&tag));
        catch!(win.set_tag(&tag, true));
        catch!(win.set_tag(&tag, false));
        catch!(win.toggle_tag(&tag));
        catch!(win.raise());
        catch!(window::begin_move(pinnacle_api::input::MouseButton::Left));
        catch!(window::begin_resize(pinnacle_api::input::MouseButton::Left));
    });

    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(1)) => (),
        err = recv.recv() => {
            anyhow::bail!(err.unwrap());
        }
    };

    window_v1.is_finished()
}
