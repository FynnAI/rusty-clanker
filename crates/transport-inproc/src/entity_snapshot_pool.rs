use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam_queue::SegQueue;
use rc_messaging::EntitySnapshot;

/// ARCH-D28's global, lock-free slot pool for `RegionTransferRequest`'s
/// `Box<EntitySnapshot>` payload. Never blocks (`acquire` always returns a usable box;
/// `release` never blocks the caller). See this blueprint's Context section for the
/// exhaustion/pre-sizing policy this type implements (this blueprint's own resolution of
/// ARCH-D28's Open Question).
pub struct EntitySnapshotPool {
    free: SegQueue<Box<EntitySnapshot>>,
    free_count: AtomicUsize,
    capacity: usize,
}

impl EntitySnapshotPool {
    /// An empty pool retaining at most `capacity` released slots for reuse.
    pub fn new(capacity: usize) -> Self {
        Self {
            free: SegQueue::new(),
            free_count: AtomicUsize::new(0),
            capacity,
        }
    }

    /// Reuse a previously `release`d allocation (overwriting its contents with `value`)
    /// if one is available; otherwise allocate fresh via `Box::new(value)`. Never blocks.
    pub fn acquire(&self, value: EntitySnapshot) -> Box<EntitySnapshot> {
        if let Some(mut slot) = self.free.pop() {
            self.free_count.fetch_sub(1, Ordering::AcqRel);
            *slot = value;
            slot
        } else {
            Box::new(value)
        }
    }

    /// Return a consumed slot for reuse. Dropped instead of retained if the pool already
    /// holds `capacity` free slots.
    pub fn release(&self, slot: Box<EntitySnapshot>) {
        loop {
            let current = self.free_count.load(Ordering::Acquire);
            if current >= self.capacity {
                return;
            }
            if self
                .free_count
                .compare_exchange_weak(current, current + 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                self.free.push(slot);
                return;
            }
        }
    }

    /// Current (best-effort — see Context's concurrency note) count of free, reusable
    /// slots. Never exceeds `capacity`. Test/diagnostic use.
    pub fn free_count(&self) -> usize {
        self.free_count.load(Ordering::Acquire)
    }
}
