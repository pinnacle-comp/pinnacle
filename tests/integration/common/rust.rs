use std::thread::JoinHandle;

use anyhow::anyhow;

#[tokio::main]
async fn run_rust_inner(run: impl FnOnce() + Send + 'static) {
    pinnacle_api::connect().await.unwrap();

    run();
}

pub fn run_rust(run: impl FnOnce() + Send + 'static) -> anyhow::Result<()> {
    std::thread::spawn(|| {
        run_rust_inner(run);
    })
    .join()
    .map_err(|_| anyhow!("rust oneshot api calls failed"))
}

#[tokio::main]
async fn setup_rust_inner(run: impl FnOnce() + Send + 'static) {
    pinnacle_api::connect().await.unwrap();

    run();

    pinnacle_api::block().await;
}

pub fn setup_rust(run: impl FnOnce() + Send + 'static) -> JoinHandle<()> {
    std::thread::spawn(|| {
        setup_rust_inner(run);
    })
}
