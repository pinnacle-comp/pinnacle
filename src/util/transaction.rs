//! A blocker-based transaction implementation.
//!
//! Based on [Niri's implementation](https://github.com/YaLTeR/niri/blob/abac28a65c6c742114ef292221dd26e2a3a2f04b/src/utils/transaction.rs).

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc, Mutex, Weak,
    },
    time::{Duration, Instant},
};

use smithay::{
    reexports::{
        calloop::{
            self,
            ping::Ping,
            timer::{TimeoutAction, Timer},
            LoopHandle,
        },
        wayland_server::Client,
    },
    utils::{IsAlive, Logical, Point, Serial},
    wayland::compositor::{Blocker, BlockerState},
};
use tracing::{error, trace, trace_span, warn};

use crate::{
    state::{State, WithState},
    window::{UnmappingWindow, WindowElement},
};

/// Timeout before a transaction is considered finished.
///
/// Prevents windows form hanging.
const TIMEOUT: Duration = Duration::from_millis(150);

/// A builder for transactions.
#[derive(Debug)]
pub struct TransactionBuilder {
    inner: Arc<Inner>,
    deadline: Rc<RefCell<Deadline>>,
    map_tos: HashMap<WindowElement, Point<i32, Logical>>,
    is_swap: bool,
}

/// A pending transaction that contains the target locations of windows once they finish
/// committing.
#[derive(Debug)]
pub struct PendingTransaction {
    /// The windows in this transactions along with their target locations.
    pub target_locs: HashMap<WindowElement, Point<i32, Logical>>,
    inner: Weak<Inner>,
    /// Whether this transaction was for a window swap.
    ///
    /// This is used to throttle swapping until the transaction finishes
    /// to prevent rapid window swaps.
    pub is_swap: bool,
    /// Held until this transaction drops, at which point the `UnmappingWindow`s in the
    /// z_index_stack are no longer valid
    _unmapping: Vec<Rc<UnmappingWindow>>,
}

impl PendingTransaction {
    /// Whether this transaction has finished.
    pub fn is_completed(&self) -> bool {
        self.inner
            .upgrade()
            .is_none_or(|inner| inner.is_completed())
    }

    /// Whether this transaction is now invalid due to a window disappearing.
    pub fn is_cancelled(&self) -> bool {
        !self.is_completed() && self.target_locs.keys().any(|win| !win.alive())
    }
}

impl TransactionBuilder {
    /// Creates a new `TransactionBuilder`.
    ///
    /// `is_swap` determines whether swapping will be unthrottled once this
    /// transaction finishes.
    pub fn new(is_swap: bool) -> Self {
        Self {
            inner: Arc::new(Inner::new()),
            deadline: Rc::new(RefCell::new(Deadline::NotRegistered(
                Instant::now() + TIMEOUT,
            ))),
            map_tos: Default::default(),
            is_swap,
        }
    }

    /// Adds a window to this transaction.
    pub fn add(
        &mut self,
        window: &WindowElement,
        target_loc: Point<i32, Logical>,
        serial: Option<Serial>,
        loop_handle: &LoopHandle<'static, State>,
    ) {
        if let Some(serial) = serial {
            let txn = Transaction {
                inner: self.inner.clone(),
                deadline: self.deadline.clone(),
            };
            // While niri schedules the timer on a case-by-case basis,
            // doing it here unconditionally is easier, albeit less performant
            txn.register_deadline_timer(loop_handle);
            window.with_state_mut(|state| state.pending_transactions.push((serial, txn)));
        }

        self.map_tos.insert(window.clone(), target_loc);
    }

    /// Consumes this transaction builder and returns a pending transaction containing
    /// added windows and their target locations.
    pub fn into_pending(self, unmapping: Vec<Rc<UnmappingWindow>>) -> PendingTransaction {
        PendingTransaction {
            target_locs: self.map_tos,
            inner: Arc::downgrade(&self.inner),
            is_swap: self.is_swap,
            _unmapping: unmapping,
        }
    }
}

/// A transaction for a window.
#[derive(Debug)]
pub struct Transaction {
    inner: Arc<Inner>,
    deadline: Rc<RefCell<Deadline>>,
}

/// A blocker that prevents a window's state from being merged until a
/// transaction finishes.
pub struct TransactionBlocker {
    inner: Weak<Inner>,
}

#[derive(Debug)]
enum Deadline {
    /// The deadline to complete a transaction has not yet been scheduled on the event loop.
    NotRegistered(Instant),
    /// The deadline has been scheduled.
    Registered {
        /// The ping that removes the timer source.
        ///
        /// I believe this is to prevent calloop from getting overwhelmed with sources.
        remove: Ping,
    },
}

#[derive(Debug)]
struct Inner {
    completed: AtomicBool,
    notifications: Mutex<Option<(Sender<Client>, Vec<Client>)>>,
}

impl Transaction {
    /// Gets a blocker for this transaction.
    pub fn blocker(&self) -> TransactionBlocker {
        trace!(transaction = ?Arc::as_ptr(&self.inner), "generating blocker");
        TransactionBlocker {
            inner: Arc::downgrade(&self.inner),
        }
    }

    /// Adds a notification for when this transaction completes.
    pub fn add_notification(&self, sender: Sender<Client>, client: Client) {
        if self.is_completed() {
            error!("tried to add notification to a completed transaction");
            return;
        }

        let mut guard = self.inner.notifications.lock().unwrap();
        guard.get_or_insert((sender, Vec::new())).1.push(client);
    }

    /// Registers this transaction's deadline timer on an event loop.
    fn register_deadline_timer<T: 'static>(&self, loop_handle: &LoopHandle<'static, T>) {
        let mut cell = self.deadline.borrow_mut();
        if let Deadline::NotRegistered(deadline) = *cell {
            let timer = Timer::from_deadline(deadline);
            let inner = Arc::downgrade(&self.inner);
            let token = loop_handle
                .insert_source(timer, move |_, _, _| {
                    let _span = trace_span!("deadline timer", transaction = ?Weak::as_ptr(&inner))
                        .entered();

                    if let Some(inner) = inner.upgrade() {
                        inner.complete();
                    } else {
                        // We should remove the timer automatically. But this callback can still
                        // just happen to run while the ping callback is scheduled, leading to this
                        // branch being legitimately taken.
                        trace!("transaction completed without removing the timer");
                    }

                    TimeoutAction::Drop
                })
                .unwrap();

            // Add a ping source that will be used to remove the timer automatically.
            let (ping, source) = calloop::ping::make_ping().unwrap();
            loop_handle
                .insert_source(source, {
                    let loop_handle = loop_handle.clone();
                    move |_, _, _| {
                        loop_handle.remove(token);
                    }
                })
                .unwrap();

            *cell = Deadline::Registered { remove: ping };
        }
    }

    /// Returns whether this transaction has already completed.
    pub fn is_completed(&self) -> bool {
        self.inner.is_completed()
    }

    /// Returns whether this is the last instance of this transaction.
    pub fn is_last(&self) -> bool {
        Arc::strong_count(&self.inner) == 1
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        let _span = trace_span!("drop", transaction = ?Arc::as_ptr(&self.inner)).entered();

        if self.is_last() {
            // If this was the last transaction, complete it.
            trace!("last transaction dropped, completing");
            self.inner.complete();

            // Also remove the timer.
            if let Deadline::Registered { remove } = &*self.deadline.borrow() {
                remove.ping();
            };
        }
    }
}

impl Blocker for TransactionBlocker {
    fn state(&self) -> BlockerState {
        if self.inner.upgrade().is_none_or(|x| x.is_completed()) {
            BlockerState::Released
        } else {
            BlockerState::Pending
        }
    }
}

impl Inner {
    fn new() -> Self {
        Self {
            completed: AtomicBool::new(false),
            notifications: Mutex::new(None),
        }
    }

    fn is_completed(&self) -> bool {
        self.completed.load(Ordering::Relaxed)
    }

    fn complete(&self) {
        self.completed.store(true, Ordering::Relaxed);

        let mut guard = self.notifications.lock().unwrap();
        if let Some((sender, clients)) = guard.take() {
            for client in clients {
                if let Err(err) = sender.send(client) {
                    warn!("error sending blocker notification: {err:?}");
                };
            }
        }
    }
}
