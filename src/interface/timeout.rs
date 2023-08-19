use core::fmt;
use std::{
    error::Error,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;

pub async fn timeout<T, F, D>(future: F, delay: D) -> Result<T, TimeoutError>
where
    F: Future<Output = T>,
    D: Future,
{
    TimeoutFuture::new(future, delay).await
}

pin_project! {
    pub struct TimeoutFuture<F, D> {
        #[pin]
        future: F,
        #[pin]
        delay: D,
    }
}

impl<F, D> TimeoutFuture<F, D> {
    #[allow(dead_code)]
    pub(super) fn new(future: F, delay: D) -> TimeoutFuture<F, D> {
        TimeoutFuture { future, delay }
    }
}

impl<F: Future, D: Future> Future for TimeoutFuture<F, D> {
    type Output = Result<F::Output, TimeoutError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.future.poll(cx) {
            Poll::Ready(v) => Poll::Ready(Ok(v)),
            Poll::Pending => match this.delay.poll(cx) {
                Poll::Ready(_) => Poll::Ready(Err(TimeoutError { _private: () })),
                Poll::Pending => Poll::Pending,
            },
        }
    }
}

/// An error returned when a future times out.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimeoutError {
    _private: (),
}

impl Error for TimeoutError {}

impl fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "future has timed out".fmt(f)
    }
}

pub trait TimerFutureExt
where
    Self: Sized,
{
    fn timeout<D>(self, delay: D) -> TimeoutFuture<Self, D>;
}

impl<F> TimerFutureExt for F
where
    F: Future,
{
    fn timeout<D>(self, delay: D) -> TimeoutFuture<Self, D> {
        TimeoutFuture {
            future: self,
            delay,
        }
    }
}
