// #![warn(missing_docs)]

use pinnacle_api_defs::pinnacle::{
    input::v0alpha1::input_service_client::InputServiceClient,
    output::v0alpha1::output_service_client::OutputServiceClient,
    process::v0alpha1::process_service_client::ProcessServiceClient,
    tag::v0alpha1::tag_service_client::TagServiceClient,
    v0alpha1::pinnacle_service_client::PinnacleServiceClient,
    window::v0alpha1::window_service_client::WindowServiceClient,
};
use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;

pub mod input;
pub mod output;
pub mod process;
pub mod tag;
pub mod util;
pub mod window;

pub fn setup() -> Result<(), Box<dyn std::error::Error>> {
    let channel = futures_lite::future::block_on(connect())?;

    let pinnacle_client = PinnacleServiceClient::new(channel.clone());
    let window_client = WindowServiceClient::new(channel.clone());
    let input_client = InputServiceClient::new(channel.clone());
    let output_client = OutputServiceClient::new(channel.clone());
    let tag_client = TagServiceClient::new(channel.clone());
    let process_client = ProcessServiceClient::new(channel.clone());

    Ok(())
}

async fn connect() -> Result<Channel, Box<dyn std::error::Error>> {
    Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(|_: Uri| {
            UnixStream::connect(std::env::var("PINNACLE_GRPC_SOCKET").unwrap())
        }))
        .await
        .map_err(|err| err.into())
}
