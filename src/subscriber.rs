use crate::event::EventRead;
use crate::ring_buffer::RingBuffer;
use crate::sequence::Sequence;
use crossbeam::sync::{Parker, Unparker};
use std::sync::Arc;

pub(crate) trait SubscriberAlert {
    fn alert(&self);
}

impl SubscriberAlert for Unparker {
    fn alert(&self) {
        self.unpark();
    }
}

/// A handle to receive events that were subscribed to from the event-bus
///
/// The [`Subscriber`] will not receive intended events that were published to the event-bus
/// before time of subscription. It will only receive intended events that are published after the
/// time of subscription, as they will have a higher sequence number than the Subscriber's internal
/// sequence value.
///
/// # Example
///
/// Basic usage:
///
/// ```ignore
/// let eventbus = Eventador::new(4)?;
///
/// // subscribe first, before publishing!
/// let subscriber = eventbus.subscribe::<usize>();
///
/// let mut i: usize = 1234;
/// eventbus.publish(i);
///
/// let mut msg = subscriber.recv().unwrap();
/// assert_eq!(i, *msg);
/// ```
///
pub struct Subscriber<T> {
    ring: Arc<RingBuffer>,
    sequence: Arc<Sequence>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: 'static> Subscriber<T>
where
    T: Send,
{
    pub(crate) fn new(ring: Arc<RingBuffer>, sequence: Arc<Sequence>) -> Self {
        Self {
            ring,
            sequence,
            _marker: std::marker::PhantomData,
        }
    }

    /// Get the current internal sequence number for the [`Subscriber`]
    ///
    /// This sequence number signifies what events the Subscriber may have already read, and any
    /// events with a sequence value higher than this are events that are still unread.
    pub fn sequence(&self) -> u64 {
        self.sequence.get()
    }

    /// Synchronously read an event of the correct type from the event-bus
    ///
    /// # Example
    ///
    /// Basic usage:
    ///
    /// ```ignore
    /// let eventbus = Eventador::new(4)?;
    /// let subscriber = eventbus.subscribe::<usize>();
    ///
    /// let mut i: usize = 1234;
    /// eventbus.publish(i);
    ///
    /// let mut msg = subscriber.recv().unwrap();
    /// assert_eq!(i, *msg);
    /// ```
    ///
    pub fn recv<'b>(&self) -> EventRead<'b, T> {
        loop {
            let sequence = self.sequence.get();

            let envelope = self
                .ring
                .get_envelope(sequence)
                .expect("ring buffer was not pre-populated with empty event envelopes")
                .clone();

            let envelope_sequence = envelope.sequence();
            if sequence == envelope_sequence {
                self.sequence.increment();
                let event_opt: Option<EventRead<T>> = unsafe { envelope.read() };

                if let Some(event) = event_opt {
                    return event;
                }
            } else if sequence > envelope_sequence {
                let parker = Parker::new();
                envelope.add_subscriber(Box::new(parker.unparker().clone()));

                parker.park();
            } else {
                self.sequence.increment(); // @todo you get here when publisher overwrites an event that has not been read yet
            }
        }
    }
}
