// #![warn(missing_docs)]

use std::sync::OnceLock;

use futures::{
    channel::mpsc::UnboundedReceiver, future::BoxFuture, stream::FuturesUnordered, Future,
    StreamExt,
};
use input::Input;
use output::Output;
use pinnacle::Pinnacle;
use process::Process;
use tag::Tag;
use tonic::transport::{Endpoint, Uri};
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
pub use xkbcommon;

static PINNACLE: OnceLock<Pinnacle> = OnceLock::new();
static PROCESS: OnceLock<Process> = OnceLock::new();
static WINDOW: OnceLock<Window> = OnceLock::new();
static INPUT: OnceLock<Input> = OnceLock::new();
static OUTPUT: OnceLock<Output> = OnceLock::new();
static TAG: OnceLock<Tag> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct ApiModules {
    pub pinnacle: &'static Pinnacle,
    pub process: &'static Process,
    pub window: &'static Window,
    pub input: &'static Input,
    pub output: &'static Output,
    pub tag: &'static Tag,
}

pub fn connect(
) -> Result<(ApiModules, UnboundedReceiver<BoxFuture<'static, ()>>), Box<dyn std::error::Error>> {
    println!("BEFORE CONNECT");
    let channel = block_on(async {
        Endpoint::try_from("http://[::]:50051")? // port doesn't matter, we use a unix socket
            .connect_with_connector(service_fn(|_: Uri| {
                println!("BEFORE UnixStream CONNECT");
                tokio::net::UnixStream::connect(
                    std::env::var("PINNACLE_GRPC_SOCKET")
                        .expect("PINNACLE_GRPC_SOCKET was not set; is Pinnacle running?"),
                )
                // .map(|stream| stream.map(|stream| stream.compat()))
            }))
            .await
    })?;

    println!("AFTER CONNECT");

    let (fut_sender, fut_recv) = futures::channel::mpsc::unbounded::<BoxFuture<()>>();

    let output = Output::new(channel.clone(), fut_sender.clone());

    let pinnacle = PINNACLE.get_or_init(|| Pinnacle::new(channel.clone()));
    let process = PROCESS.get_or_init(|| Process::new(channel.clone(), fut_sender.clone()));
    let window = WINDOW.get_or_init(|| Window::new(channel.clone()));
    let input = INPUT.get_or_init(|| Input::new(channel.clone(), fut_sender.clone()));
    let tag = TAG.get_or_init(|| Tag::new(channel.clone(), fut_sender.clone()));
    let output = OUTPUT.get_or_init(|| output);

    let modules = ApiModules {
        pinnacle,
        process,
        window,
        input,
        output,
        tag,
    };

    Ok((modules, fut_recv))
}

pub async fn listen(
    fut_recv: UnboundedReceiver<BoxFuture<'static, ()>>, // api_modules: ApiModules<'a>,
) {
    let mut future_set = FuturesUnordered::<
        BoxFuture<(
            Option<BoxFuture<()>>,
            Option<UnboundedReceiver<BoxFuture<()>>>,
        )>,
    >::new();

    future_set.push(Box::pin(async move {
        let (fut, stream) = fut_recv.into_future().await;
        (fut, Some(stream))
    }));

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
}

pub(crate) fn block_on<F: Future>(fut: F) -> F::Output {
    futures::executor::block_on(fut)
    // tokio::task::block_in_place(|| futures::executor::block_on(fut))
}
