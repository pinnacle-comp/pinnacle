mod backend;
mod grab;
mod handlers;
mod input;
mod layout;
mod pointer;
mod state;
mod tag;
mod window;
mod xdg;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    crate::backend::winit::run_winit()?;
    Ok(())
}
