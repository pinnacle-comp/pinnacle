pub mod output;
pub mod pinnacle;
pub mod tag;

use ::pinnacle::api::TonicResult;
use tonic::Status;

#[macro_export]
macro_rules! start_test_grpc_server {
    ($path:expr, $service:expr) => {{
        let uds = ::tokio::net::UnixListener::bind(&$path).unwrap();
        let uds_stream = ::tokio_stream::wrappers::UnixListenerStream::new(uds);

        ::std::env::set_var("PINNACLE_GRPC_SOCKET", $path);

        let grpc_server = ::tonic::transport::Server::builder().add_service($service);

        ::tokio::spawn(async move {
            grpc_server.serve_with_incoming(uds_stream).await.unwrap();
        })
    }};
}

#[macro_export]
macro_rules! test_body {
    ($method:ident, $slf:ident, $req:ident) => {{
        let Some(expected) = $slf.$method.lock().unwrap().pop_front() else {
            $slf.sender
                .send(format!(
                    "no expected request at {}:{}:{}",
                    file!(),
                    line!(),
                    column!()
                ))
                .unwrap();

            return $crate::common::grpc::test_result();
        };

        let request = $req.into_inner();

        if request != expected {
            $slf.sender
                .send(format!(
                    "request != expected, {request:?}, {expected:?} at {}:{}:{}",
                    file!(),
                    line!(),
                    column!()
                ))
                .unwrap();
        }

        $crate::common::grpc::test_result()
    }};
}

fn test_result<T>() -> TonicResult<T> {
    Err(Status::unimplemented("test service"))
}

#[macro_export]
macro_rules! gen_test_infra {
    (
        name = $struct_name:ident,
        service = $srv_name:ty,
        assoc_tys = {
            $(type $assoc:ident = $assoc_ty:ty;)*
        },
        unary = {
            $($method:ident($req:ty) -> $resp:ty),* $(,)?
        },
        other = {
            $($other_method:ident($other_req:ty) -> $other_resp:ty),* $(,)?
        }$(,)?
    ) => {
        #[derive(Clone)]
        pub struct $struct_name {
            $(
                pub $method: ::std::sync::Arc<::std::sync::Mutex<::std::collections::VecDeque<$req>>>,
            )*
            sender: ::tokio::sync::mpsc::UnboundedSender<::std::string::String>,
        }

        impl $struct_name {
            pub fn new() -> (Self, ::tokio::sync::mpsc::UnboundedReceiver<::std::string::String>) {
                let (sender, recv) = ::tokio::sync::mpsc::unbounded_channel();
                let this = Self {
                    $($method: ::std::default::Default::default(),)*
                    sender,
                };
                (this, recv)
            }

            pub fn is_finished(&self) -> ::anyhow::Result<()> {
                $(
                    let $method = self.$method.lock().unwrap();
                    ::anyhow::ensure!($method.is_empty(), "{} not finished: {:?}", stringify!($method), &*$method);
                )*
                Ok(())
            }

            $(
                pub fn $method(&self, expected: impl IntoIterator<Item = $req>) {
                    *self.$method.lock().unwrap() = expected.into_iter().collect();
                }
            )*
        }

        #[::tonic::async_trait]
        impl $srv_name for $struct_name {
            $(
                type $assoc = $assoc_ty;
            )*

            $(
                async fn $method(&self, request: ::tonic::Request<$req>) -> ::pinnacle::api::TonicResult<$resp> {
                    $crate::test_body!($method, self, request)
                }
            )*

            $(
                async fn $other_method(&self, _request: ::tonic::Request<$other_req>) -> ::pinnacle::api::TonicResult<$other_resp> {
                    $crate::common::grpc::test_result()
                }
            )*
        }
    };
}

// use crate::test_body;
//
// #[tonic::async_trait]
// impl pinnacle_api_defs::pinnacle::tag::v1::tag_service_server::TagService for TagService {
//     async fn get(&self, request: Request<GetRequest>) -> TonicResult<GetResponse> {
//         test_body!(get, self, request)
//     }
//
//     async fn get_active(
//         &self,
//         request: Request<GetActiveRequest>,
//     ) -> TonicResult<GetActiveResponse> {
//         test_body!(get_active, self, request)
//     }
//
//     async fn get_name(&self, request: Request<GetNameRequest>) -> TonicResult<GetNameResponse> {
//         test_body!(get_name, self, request)
//     }
//
//     async fn get_output_name(
//         &self,
//         request: Request<GetOutputNameRequest>,
//     ) -> TonicResult<GetOutputNameResponse> {
//         test_body!(get_output_name, self, request)
//     }
//
//     async fn set_active(&self, request: Request<SetActiveRequest>) -> TonicResult<()> {
//         test_body!(set_active, self, request)
//     }
//
//     async fn switch_to(&self, request: Request<SwitchToRequest>) -> TonicResult<()> {
//         test_body!(switch_to, self, request)
//     }
//
//     async fn add(&self, request: Request<AddRequest>) -> TonicResult<AddResponse> {
//         test_body!(add, self, request)
//     }
//
//     async fn remove(&self, request: Request<RemoveRequest>) -> TonicResult<()> {
//         test_body!(remove, self, request)
//     }
// }
