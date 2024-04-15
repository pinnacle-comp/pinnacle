use std::{collections::HashMap, sync::{atomic::AtomicBool, Arc}};

use smithay::reexports::wayland_server::Client;

#[derive(Default)]
pub struct Wlcs {
    pub clients: HashMap<i32, Client>,
    pub running: Arc<AtomicBool>,
}
