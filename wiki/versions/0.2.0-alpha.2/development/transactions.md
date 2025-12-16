# Transactions

> [!IMPORTANT]
> This information may be out of date if I forget to update it.

**Transactions** are used to synchronize the updating of window sizes and their
position in the global space. They are essential to ensuring that
Every Frame is Perfect<sup>TM</sup>.

For example, take the tiled layout. When a window opens, other windows may
need to resize and shift around to accommodate the new window,
demonstrated by the following sequence of events:
1. A window opens.
2. Pinnacle requests a layout from the config.
3. Once the layout is received, Pinnacle configures all visible tiled windows,
   sending updated sizes.
4. Pinnacle must then map tiled windows to their new location in the global space.

There is a problem with 3.: windows do not resize at the same time.
If we were to configure window sizes and immediately map, you would see all
windows jump to their new location at the same size, then see their sizes
update at possibly different times. Obviously this doesn't look great.

To solve this, we have blocker-based transactions (based off of Niri's implementation
because I tried to do this twice in the past but my brain wasn't wrinkly enough).

## Blockers

To understand the implementation, it's best to understand what a blocker is.

A [blocker](https://smithay.github.io/smithay/smithay/wayland/compositor/trait.Blocker.html)
prevents state changes from being merged into the current state when attached
to a client. When used correctly, they allow us to "hold" windows at their current state
until unblocked.

## Usage

At the core of Pinnacle's transaction implementation is the `TransactionBuilder`.
This struct builds transactions by allowing you to add windows, their target locations,
and optional serials from configures.

The general usage is:
1. Create a `TransactionBuilder` with `TransactionBuilder::new()`.
2. For a set of windows that you want to synchronize the update of,
    1. Configure their new size, keeping the returned serial.
    2. Call `TransactionBuilder::add` on the builder, giving it
       the window, its destination loc after updating, and the optional serial.
3. Call `TransactionBuilder::into_pending` on the builder when done.
   This takes a vec of `UnmappingWindow`s for rendering unmap snapshots,
   but we can ignore it for this page. Store the returned `PendingTransaction`
   somewhere so you can check if it's done.
4. Check if the `PendingTransaction` is done. If it is, you can access
   the contained windows and target locations to map the windows. We
   do this at the end of every event loop cycle.

Note: this has been adapted from Niri's implementation to fit Pinnacle's needs,
namely that we use a global space and have to deal with mapping windows
to locations. This is why there's a `PendingTransaction` struct.

## Implementation

All windows store a vec of `Transaction`s (not to be confused with the
`PendingTransaction` above) and serials. When a `TransactionBuilder`
is created, it creates its own `Transaction`. When windows are added
to the builder, that `Transaction` is cloned and pushed to each window's
vec along with the accompanying serial.

`Transaction`s and their clones refer to the same inner transaction.
When all `Transaction`s referencing a certain transaction drop, or when
a timeout is reached, the transaction completes, causing
`PendingTransaction::is_completed` to return true.

We add a pre-commit hook to all mapped windows that checks stored `Transactions`
against the currently committed serial. The hook takes the most
recently committed `Transaction`, dropping previous ones to free them.
If that `Transaction` isn't done, a blocker is added to the window's client.
This is what allows us to synchronize updates.
