mod common;

use std::time::Duration;

use common::{
    grpc::{pinnacle, tag},
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
