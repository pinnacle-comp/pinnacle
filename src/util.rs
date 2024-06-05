use std::sync::atomic::{AtomicU64, Ordering};

use smithay::reexports::rustix::process::{getrlimit, setrlimit, Resource, Rlimit};
use tracing::warn;

static NOFILE_RLIMIT_CURRENT: AtomicU64 = AtomicU64::new(0);
static NOFILE_RLIMIT_MAXIMUM: AtomicU64 = AtomicU64::new(0);

pub fn increase_nofile_rlimit() {
    let mut limits = getrlimit(Resource::Nofile);

    NOFILE_RLIMIT_CURRENT.store(limits.current.unwrap_or(0), Ordering::SeqCst);
    NOFILE_RLIMIT_MAXIMUM.store(limits.maximum.unwrap_or(0), Ordering::SeqCst);

    limits.current = limits.maximum;

    if let Err(err) = setrlimit(Resource::Nofile, limits) {
        warn!("Failed to raise nofile limit: {err}");
    }
}

pub fn restore_nofile_rlimit() {
    let current = NOFILE_RLIMIT_CURRENT.load(Ordering::SeqCst);
    let maximum = NOFILE_RLIMIT_MAXIMUM.load(Ordering::SeqCst);

    let limits = Rlimit {
        current: (current > 0).then_some(current),
        maximum: (maximum > 0).then_some(maximum),
    };

    if let Err(err) = setrlimit(Resource::Nofile, limits) {
        warn!("Failed to restore nofile limit: {err}");
    }
}
