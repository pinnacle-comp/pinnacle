pub mod convert;

pub trait BlockOnTokio {
    type Output;

    fn block_on_tokio(self) -> Self::Output;
}

impl<F: Future> BlockOnTokio for F {
    type Output = F::Output;

    /// Blocks on a future using the current Tokio runtime.
    fn block_on_tokio(self) -> Self::Output {
        tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(self)
        })
    }
}
