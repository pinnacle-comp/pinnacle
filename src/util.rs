pub mod treediff;

use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use smithay::{
    reexports::rustix::process::{getrlimit, setrlimit, Resource, Rlimit},
    utils::{Point, Rectangle, Size},
};
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

#[inline(never)]
pub fn cause_panic() {
    let a = Duration::from_secs(1);
    let b = Duration::from_secs(2);
    let _ = a - b;
}

/// Returns the locaation that centers the given `size` within a `rect`.
pub fn centered_loc<Kind>(rect: Rectangle<i32, Kind>, size: Size<i32, Kind>) -> Point<i32, Kind> {
    Point::from((
        rect.loc.x + rect.size.w / 2 - size.w / 2,
        rect.loc.y + rect.size.h / 2 - size.h / 2,
    ))
}

/// Runs a closure every time the given duration passes with the amount of times
/// this has been called since the last time the closure has run.
///
/// # Usage
/// ```
/// # use pinnacle::executions_per_duration;
/// # use std::time::Duration;
///
/// fn count_me() {
///     executions_per_duration!(Duration::from_secs(1), |amt| {
///         println!("count_me has been called {amt} times in the last second.")
///     });
/// }
/// ```
#[macro_export]
macro_rules! executions_per_duration {
    ($duration:expr, $closure:expr) => {{
        static COUNTER: ::std::sync::atomic::AtomicUsize = ::std::sync::atomic::AtomicUsize::new(0);
        static TIME: ::std::sync::Mutex<::std::option::Option<::std::time::Instant>> =
            ::std::sync::Mutex::new(None);
        let mut then = TIME.lock().unwrap();
        let then = then.get_or_insert(::std::time::Instant::now());
        if then.elapsed() > $duration {
            let counter = COUNTER.load(std::sync::atomic::Ordering::Relaxed);
            ($closure)(counter);
            *then = ::std::time::Instant::now();
            COUNTER.store(0, ::std::sync::atomic::Ordering::Relaxed);
        }
        COUNTER.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed);
    }};
}
