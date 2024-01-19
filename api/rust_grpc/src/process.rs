use pinnacle_api_defs::pinnacle::process::v0alpha1::{
    process_service_client::ProcessServiceClient, SpawnRequest,
};
use tokio_stream::StreamExt;
use tonic::transport::Channel;

use crate::FutSender;

#[derive(Debug, Clone)]
pub struct Process {
    client: ProcessServiceClient<Channel>,
    fut_sender: FutSender,
}

pub struct SpawnCallbacks {
    pub stdout: Option<Box<dyn FnMut(String) + Send>>,
    pub stderr: Option<Box<dyn FnMut(String) + Send>>,
    pub exit: Option<Box<dyn FnMut(Option<i32>, String) + Send>>,
}

impl Process {
    pub fn new(client: ProcessServiceClient<Channel>, fut_sender: FutSender) -> Process {
        Self { client, fut_sender }
    }

    pub fn spawn(&self, args: impl IntoIterator<Item = impl Into<String>>) {
        self.spawn_inner(args, false, None);
    }

    pub fn spawn_with_callbacks(
        &self,
        args: impl IntoIterator<Item = impl Into<String>>,
        callbacks: SpawnCallbacks,
    ) {
        self.spawn_inner(args, false, Some(callbacks));
    }

    pub fn spawn_once(&self, args: impl IntoIterator<Item = impl Into<String>>) {
        self.spawn_inner(args, true, None);
    }

    pub fn spawn_once_with_callbacks(
        &self,
        args: impl IntoIterator<Item = impl Into<String>>,
        callbacks: SpawnCallbacks,
    ) {
        self.spawn_inner(args, true, Some(callbacks));
    }

    fn spawn_inner(
        &self,
        args: impl IntoIterator<Item = impl Into<String>>,
        once: bool,
        callbacks: Option<SpawnCallbacks>,
    ) {
        let mut client = self.client.clone();

        let args = args
            .into_iter()
            .map(|arg| Into::<String>::into(arg))
            .collect::<Vec<_>>();

        let request = SpawnRequest {
            args,
            once: Some(once),
            has_callback: Some(callbacks.is_some()),
        };

        self.fut_sender
            .send(Box::pin(async move {
                let mut stream = client.spawn(request).await.unwrap().into_inner();
                let Some(mut callbacks) = callbacks else { return };
                while let Some(Ok(response)) = stream.next().await {
                    if let Some(line) = response.stdout {
                        if let Some(stdout) = callbacks.stdout.as_mut() {
                            stdout(line);
                        }
                    }
                    if let Some(line) = response.stderr {
                        if let Some(stderr) = callbacks.stderr.as_mut() {
                            stderr(line);
                        }
                    }
                    if let Some(exit_msg) = response.exit_message {
                        if let Some(exit) = callbacks.exit.as_mut() {
                            exit(response.exit_code, exit_msg);
                        }
                    }
                }
            }))
            .unwrap();

        // tokio::spawn(async move {
        //     let mut stream = client.spawn(request).await.unwrap().into_inner();
        //     let Some(mut callbacks) = callbacks else { return };
        //     while let Some(Ok(response)) = stream.next().await {
        //         if let Some(line) = response.stdout {
        //             if let Some(stdout) = callbacks.stdout.as_mut() {
        //                 stdout(line);
        //             }
        //         }
        //         if let Some(line) = response.stderr {
        //             if let Some(stderr) = callbacks.stderr.as_mut() {
        //                 stderr(line);
        //             }
        //         }
        //         if let Some(exit_msg) = response.exit_message {
        //             if let Some(exit) = callbacks.exit.as_mut() {
        //                 exit(response.exit_code, exit_msg);
        //             }
        //         }
        //     }
        // });
    }
}
