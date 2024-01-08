use tonic::{Request, Response, Status};

use self::request::{CloseWindowRequest, SetKeybindRequest, SetMousebindRequest};

pub mod request {
    tonic::include_proto!("request");
}

#[derive(Default)]
pub struct CommandServer;

#[tonic::async_trait]
impl request::command_service_server::CommandService for CommandServer {
    async fn set_keybind(
        &self,
        request: Request<SetKeybindRequest>,
    ) -> Result<Response<()>, Status> {
        println!("got set_keybind with {request:?}");
        Ok(Response::new(()))
    }

    async fn set_mousebind(
        &self,
        request: Request<SetMousebindRequest>,
    ) -> Result<Response<()>, Status> {
        println!("got set_mousebind with {request:?}");
        Ok(Response::new(()))
    }

    async fn close_window(
        &self,
        request: Request<CloseWindowRequest>,
    ) -> Result<Response<()>, Status> {
        println!("got close_window with {request:?}");
        Ok(Response::new(()))
    }
}
