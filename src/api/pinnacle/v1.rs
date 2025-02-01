use pinnacle_api_defs::pinnacle::{
    self,
    v1::{
        self, BackendRequest, BackendResponse, KeepaliveRequest, KeepaliveResponse, QuitRequest,
        ReloadConfigRequest, SetXwaylandClientSelfScaleRequest,
    },
};
use tonic::{Request, Streaming};
use tracing::{info, trace};

use crate::api::{
    run_bidirectional_streaming, run_unary, run_unary_no_response, ResponseStream, TonicResult,
};

#[tonic::async_trait]
impl v1::pinnacle_service_server::PinnacleService for super::PinnacleService {
    type KeepaliveStream = ResponseStream<KeepaliveResponse>;

    async fn quit(&self, _request: Request<QuitRequest>) -> TonicResult<()> {
        trace!("PinnacleService.quit");

        run_unary_no_response(&self.sender, |state| {
            state.pinnacle.shutdown();
        })
        .await
    }

    async fn reload_config(&self, _request: Request<ReloadConfigRequest>) -> TonicResult<()> {
        run_unary_no_response(&self.sender, |state| {
            info!("Reloading config");
            state
                .pinnacle
                .start_config(false)
                .expect("failed to restart config");
        })
        .await
    }

    async fn keepalive(
        &self,
        _request: Request<Streaming<KeepaliveRequest>>,
    ) -> TonicResult<Self::KeepaliveStream> {
        run_bidirectional_streaming(
            self.sender.clone(),
            _request.into_inner(),
            |_, _| {},
            |state, sender, _| {
                let (oneshot_tx, oneshot_rx) = tokio::sync::oneshot::channel::<()>();
                state.pinnacle.config.keepalive_sender.replace(oneshot_tx);
                tokio::spawn(async move {
                    let _sender = sender;
                    let _ = oneshot_rx.await;
                    // sender should drop here and kill the config
                });
            },
        )
    }

    async fn backend(&self, _request: Request<BackendRequest>) -> TonicResult<BackendResponse> {
        run_unary(&self.sender, |state| {
            let backend = match &state.backend {
                crate::backend::Backend::Winit(_) => pinnacle::v1::Backend::Window,
                crate::backend::Backend::Udev(_) => pinnacle::v1::Backend::Tty,
                #[cfg(feature = "testing")]
                crate::backend::Backend::Dummy(_) => pinnacle::v1::Backend::Tty, // unused
            };

            let mut response = BackendResponse::default();
            response.set_backend(backend);

            Ok(response)
        })
        .await
    }

    async fn set_xwayland_client_self_scale(
        &self,
        request: Request<SetXwaylandClientSelfScaleRequest>,
    ) -> TonicResult<()> {
        let should_self_scale = request.into_inner().self_scale;

        run_unary_no_response(&self.sender, move |state| {
            if let Some(xwayland_state) = state.pinnacle.xwayland_state.as_mut() {
                xwayland_state.should_clients_self_scale = should_self_scale;
                state.pinnacle.update_xwayland_scale();
            }
        })
        .await
    }
}
