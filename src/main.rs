mod api;
mod backend;
mod cursor;
mod focus;
mod grab;
mod handlers;
mod input;
mod layout;
mod pointer;
mod render;
mod state;
mod window;
mod xdg;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    match tracing_subscriber::EnvFilter::try_from_default_env() {
        Ok(env_filter) => {
            tracing_subscriber::fmt()
                .compact()
                .with_env_filter(env_filter)
                .init();
        }
        Err(_) => {
            tracing_subscriber::fmt().compact().init();
        }
    }

    let mut args = std::env::args().skip(1);
    if let Some("--winit") = args.next().as_deref() {
        tracing::info!("Starting winit backend");
        crate::backend::winit::run_winit()?;
    } else {
        tracing::info!("Starting udev backend");
        crate::backend::udev::run_udev()?;
    }

    Ok(())
}
