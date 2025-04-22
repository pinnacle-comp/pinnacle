use pinnacle_api_defs::pinnacle::process::{
    self,
    v1::{SpawnRequest, SpawnResponse, WaitOnSpawnRequest, WaitOnSpawnResponse},
};
use tonic::Request;

use crate::{
    api::{run_server_streaming, run_unary, ResponseStream, TonicResult},
    process::PipeProcesses,
};

#[tonic::async_trait]
impl process::v1::process_service_server::ProcessService for super::ProcessService {
    type WaitOnSpawnStream = ResponseStream<WaitOnSpawnResponse>;

    async fn spawn(&self, request: Request<SpawnRequest>) -> TonicResult<SpawnResponse> {
        let request = request.into_inner();

        let SpawnRequest {
            cmd,
            unique,
            once,
            shell_cmd,
            envs,
            pipe_stdin,
            pipe_stdout,
            pipe_stderr,
        } = request;

        run_unary(&self.sender, move |state| {
            let pipe_processes = !state.pinnacle.config.debug.disable_process_piping;
            let fds = state.pinnacle.process_state.spawn(
                &cmd,
                &shell_cmd,
                unique,
                once,
                envs,
                &state.pinnacle.xdg_base_dirs,
                PipeProcesses {
                    stdin: pipe_processes && pipe_stdin,
                    stdout: pipe_processes && pipe_stdout,
                    stderr: pipe_processes && pipe_stderr,
                },
            );

            Ok(SpawnResponse {
                spawn_data: fds.map(|data| process::v1::SpawnData {
                    pid: data.pid,
                    fd_socket_path: data.fd_socket_path,
                    has_stdin: data.has_stdin,
                    has_stdout: data.has_stdout,
                    has_stderr: data.has_stderr,
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
