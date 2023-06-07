mod backend;
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

    tracing::info!("Starting winit backend");
    crate::backend::winit::run_winit()?;
    Ok(())
}
