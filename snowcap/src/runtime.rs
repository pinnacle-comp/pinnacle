use std::{
    pin::Pin,
    task::{Context, Poll},
};

use smithay_client_toolkit::reexports::calloop;

pub struct CurrentTokioExecutor;

impl iced_futures::Executor for CurrentTokioExecutor {
    fn new() -> Result<Self, futures::io::Error>
    where
        Self: Sized,
    {
        Ok(Self)
    }

    fn spawn(
        &self,
        future: impl futures::prelude::Future<Output = ()> + iced_futures::MaybeSend + 'static,
    ) {
        tokio::runtime::Handle::current().spawn(future);
    }
}

pub struct CalloopSenderSink<T>(calloop::channel::Sender<T>);

impl<T> Clone for CalloopSenderSink<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> CalloopSenderSink<T> {
    pub fn new(sender: calloop::channel::Sender<T>) -> Self {
        Self(sender)
    }
}

impl<T> futures::Sink<T> for CalloopSenderSink<T> {
    type Error = futures::channel::mpsc::SendError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        let _ = self.0.send(item);

        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
