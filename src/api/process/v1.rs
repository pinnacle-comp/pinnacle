use pinnacle_api_defs::pinnacle::process::{
    self,
    v1::{SpawnRequest, SpawnResponse, WaitOnSpawnRequest, WaitOnSpawnResponse},
};
use tonic::Request;

use crate::api::{run_server_streaming, run_unary, ResponseStream, TonicResult};

#[tonic::async_trait]
impl process::v1::process_service_server::ProcessService for super::ProcessService {
    type WaitOnSpawnStream = ResponseStream<WaitOnSpawnResponse>;

    async fn spawn(&self, request: Request<SpawnRequest>) -> TonicResult<SpawnResponse> {
        let request = request.into_inner();

        let unique = request.unique;
        let once = request.once;
        let cmd = request.cmd;
        let shell_cmd = request.shell_cmd;
        let envs = request.envs;

        run_unary(&self.sender, move |state| {
            let fds = state
                .pinnacle
                .process_state
                .spawn(&cmd, &shell_cmd, unique, once, envs);

            Ok(SpawnResponse {
                spawn_data: fds.map(|fds| process::v1::SpawnData {
                    pid: fds.pid,
                    stdin_fd: fds.stdin,
                    stdout_fd: fds.stdout,
                    stderr_fd: fds.stderr,
                }),
            })
        })
        .await
    }

    async fn wait_on_spawn(
        &self,
        request: Request<WaitOnSpawnRequest>,
    ) -> TonicResult<Self::WaitOnSpawnStream> {
        let pid = request.into_inner().pid;

        run_server_streaming(&self.sender, move |state, sender| {
            let wait_recv = state.pinnacle.process_state.wait_on_spawn(pid);

            let Some(wait_recv) = wait_recv else {
                let _ = sender.send(Ok(WaitOnSpawnResponse {
                    exit_code: None,
                    exit_msg: None,
                }));
                return Ok(());
            };

            tokio::spawn(async move {
                let exit = wait_recv.await.ok().flatten();
                let _ = sender.send(Ok(WaitOnSpawnResponse {
                    exit_code: exit.as_ref().and_then(|exit| exit.exit_code),
                    exit_msg: exit.as_ref().and_then(|exit| exit.exit_msg.clone()),
                }));
            });

            Ok(())
        })
        .await
    }
}
