//! Work counters of this thread, for tests that bound how much work a run does by counting it
//! rather than timing it: the nodes the node-id traversal visits (`id`) and the key comparisons
//! map reads and writes make.

use std::cell::Cell;

thread_local! {
    static ID_STEPS: Cell<u64> = const { Cell::new(0) };
    static KEY_COMPARISONS: Cell<u64> = const { Cell::new(0) };
}

/// The counts of this thread since its last `reset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    /// Statements and expressions the node-id traversal visited.
    pub id_steps: u64,
    /// Key comparisons of map reads and writes.
    pub key_comparisons: u64,
}

/// Sets this thread's counts to zero.
pub fn reset() {
    ID_STEPS.with(|c| c.set(0));
    KEY_COMPARISONS.with(|c| c.set(0));
}

/// This thread's counts.
pub fn counts() -> Counts {
    Counts {
        id_steps: ID_STEPS.with(Cell::get),
        key_comparisons: KEY_COMPARISONS.with(Cell::get),
    }
}

/// Adds `c` to this thread's counts: what a run on a thread of its own counted.
pub(crate) fn add(c: Counts) {
    ID_STEPS.with(|x| x.set(x.get() + c.id_steps));
    KEY_COMPARISONS.with(|x| x.set(x.get() + c.key_comparisons));
}

pub(crate) fn id_step() {
    ID_STEPS.with(|c| c.set(c.get() + 1));
}

pub(crate) fn key_comparison() {
    KEY_COMPARISONS.with(|c| c.set(c.get() + 1));
}
