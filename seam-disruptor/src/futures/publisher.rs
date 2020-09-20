use crate::ring_buffer::RingBuffer;
use async_channel::RecvError;
use futures::task::{Context, Poll};
use futures::Sink;
use std::pin::Pin;
use std::sync::Arc;

pub struct AsyncPublisher<T> {
    ring: Arc<RingBuffer>,
    sequence: Option<u64>,
    event: Option<T>,
}

impl<T: 'static + Unpin> AsyncPublisher<T> {
    pub fn new(ring: Arc<RingBuffer>) -> Self {
        Self {
            ring,
            sequence: None,
            event: None,
        }
    }

    pub(crate) fn publish(&mut self) {
        if let Some(sequence) = self.sequence.take() {
            if let Some(envelope) = self.ring.get_envelope(sequence) {
                if let Some(event) = self.event.take() {
                    envelope.overwrite(sequence, event);
                }
            }
        }
    }
}

impl<T: 'static + Unpin> Sink<T> for AsyncPublisher<T> {
    type Error = RecvError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.event.is_some() {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        let sequence = self.ring.next();

        self.sequence.replace(sequence);
        self.event.replace(item);

        Ok(())
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.publish();
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        drop(self);
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    // use crate::async_publisher::*;
}