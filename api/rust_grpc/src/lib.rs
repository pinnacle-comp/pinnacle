// #![warn(missing_docs)]

use std::sync::OnceLock;

use futures::{executor::block_on, future::BoxFuture, stream::FuturesUnordered, StreamExt};
use input::Input;
use output::Output;
use pinnacle::Pinnacle;
use pinnacle_api_defs::pinnacle::{
    input::v0alpha1::input_service_client::InputServiceClient,
    output::v0alpha1::output_service_client::OutputServiceClient,
    process::v0alpha1::process_service_client::ProcessServiceClient,
    tag::v0alpha1::tag_service_client::TagServiceClient,
    v0alpha1::pinnacle_service_client::PinnacleServiceClient,
    window::v0alpha1::window_service_client::WindowServiceClient,
};
use process::Process;
use tag::Tag;
use tokio::{net::UnixStream, sync::mpsc::UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;
use window::Window;

pub mod input;
pub mod output;
pub mod pinnacle;
pub mod process;
pub mod tag;
pub mod util;
pub mod window;

pub use pinnacle_api_macros::config;

static PINNACLE: OnceLock<Pinnacle> = OnceLock::new();
static PROCESS: OnceLock<Process> = OnceLock::new();
static WINDOW: OnceLock<Window> = OnceLock::new();
static INPUT: OnceLock<Input> = OnceLock::new();
static OUTPUT: OnceLock<Output> = OnceLock::new();
static TAG: OnceLock<Tag> = OnceLock::new();

pub(crate) type FutSender = UnboundedSender<BoxFuture<'static, ()>>;

pub fn create_modules(
) -> Result<(ApiModules, UnboundedReceiverStream<BoxFuture<'static, ()>>), Box<dyn std::error::Error>>
{
    let channel = connect()?;

    let pinnacle_client = PinnacleServiceClient::new(channel.clone());
    let window_client = WindowServiceClient::new(channel.clone());
    let input_client = InputServiceClient::new(channel.clone());
    let output_client = OutputServiceClient::new(channel.clone());
    let tag_client = TagServiceClient::new(channel.clone());
    let process_client = ProcessServiceClient::new(channel.clone());

    let (fut_sender, fut_receiver) = tokio::sync::mpsc::unbounded_channel::<BoxFuture<()>>();

    let fut_receiver = UnboundedReceiverStream::new(fut_receiver);

    let pinnacle = PINNACLE.get_or_init(|| Pinnacle::new(pinnacle_client));
    let process = PROCESS.get_or_init(|| Process::new(process_client, fut_sender.clone()));
    let window = WINDOW.get_or_init(|| Window::new(window_client, tag_client.clone()));
    let input = INPUT.get_or_init(|| Input::new(input_client, fut_sender.clone()));
    let output = OUTPUT.get_or_init(|| Output::new(output_client, tag_client.clone()));
    let tag = TAG.get_or_init(|| Tag::new(tag_client));

    let modules = ApiModules {
        pinnacle,
        process,
        window,
        input,
        output,
        tag,
    };

    Ok((modules, fut_receiver))
}

pub fn listen(
    fut_receiver: UnboundedReceiverStream<BoxFuture<()>>,
    // api_modules: ApiModules<'a>,
) {
    let mut future_set = FuturesUnordered::<
        BoxFuture<(
            Option<BoxFuture<()>>,
            Option<UnboundedReceiverStream<BoxFuture<()>>>,
        )>,
    >::new();

    future_set.push(Box::pin(async move {
        let (fut, stream) = fut_receiver.into_future().await;
        (fut, Some(stream))
    }));

    block_on(async move {
        while let Some((fut, stream)) = future_set.next().await {
            if let Some(fut) = fut {
                future_set.push(Box::pin(async move {
                    fut.await;
                    (None, None)
                }));
            }
            if let Some(stream) = stream {
                future_set.push(Box::pin(async move {
                    let (fut, stream) = stream.into_future().await;
                    (fut, Some(stream))
                }))
            }
        }
    });
}

// #[derive(Debug, Clone)]
pub struct ApiModules {
    pub pinnacle: &'static Pinnacle,
    pub process: &'static Process,
    pub window: &'static Window,
    pub input: &'static Input,
    pub output: &'static Output,
    pub tag: &'static Tag,
}

pub fn connect() -> Result<Channel, Box<dyn std::error::Error>> {
    block_on(async {
        Endpoint::try_from("http://[::]:50051")? // port doesn't matter, we use a unix socket
            .connect_with_connector(service_fn(|_: Uri| {
                UnixStream::connect(
                    std::env::var("PINNACLE_GRPC_SOCKET")
                        .expect("PINNACLE_GRPC_SOCKET was not set; is Pinnacle running?"),
                )
            }))
            .await
    })
    .map_err(|err| err.into())
}
