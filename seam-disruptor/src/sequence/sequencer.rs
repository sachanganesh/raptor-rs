use crate::sequence::sequence_group::SequenceGroup;
use crate::sequence::Sequence;
use std::sync::Arc;

pub struct Sequencer {
    cursor: Sequence,
    gating_sequence_cache: Arc<Sequence>,
    gating_sequences: SequenceGroup,
    buffer_size: u64,
}

impl Sequencer {
    pub fn new(buffer_size: u64) -> Self {
        Self {
            cursor: Sequence::with_value(0),
            gating_sequence_cache: Arc::new(Sequence::with_value(0)),
            gating_sequences: SequenceGroup::new(),
            buffer_size,
        }
    }

    pub(crate) fn register_gating_sequence(&self, sequence: Arc<Sequence>) {
        self.gating_sequences.add(sequence);
    }

    pub fn get(&self) -> u64 {
        self.cursor.get()
    }

    pub fn next(&self) -> u64 {
        self.next_from(1)
            .expect("sequencer could not get next sequence number from sequence 1")
    }

    pub fn next_from(&self, n: u64) -> anyhow::Result<u64> {
        if n < 1 || n > self.buffer_size {
            return Err(anyhow::Error::msg("n must be > 0 and < buffer_size"));
        }

        loop {
            let current: u64 = self.cursor.get();
            let icurrent: i64 = current as i64;
            let next: i64 = (current + n) as i64;

            let wrap_point: i64 = next - self.buffer_size as i64;
            let cached_gating_sequence: i64 = self.gating_sequence_cache.get() as i64;

            if wrap_point > cached_gating_sequence || cached_gating_sequence > icurrent {
                let gating_sequence = self.gating_sequences.minimum_sequence(current);

                if wrap_point > gating_sequence as i64 {
                    // async_std::task::block_on(async_std::task::sleep(Duration::from_nanos(1)));
                    continue;
                }

                self.gating_sequence_cache.set(gating_sequence);
            } else if self.cursor.compare_and_swap(current, next as u64) {
                break Ok(next as u64);
            }
        }
    }
}