//! The capacity Go gives a slice that `append` grows, from the Go 1.27 runtime on amd64:
//! `nextslicecap` and `growslice` in `runtime/slice.go`, `roundupsize` in `runtime/msize.go`,
//! the size classes of `internal/runtime/gc/sizeclasses.go` and the malloc header of
//! `internal/runtime/gc/malloc.go`.
//!
//! The compiler may instead give a non-escaping slice of at most 32 bytes a backing store on the
//! stack (`cmd/compile/internal/ssagen`, `VariableMakeThreshold`), whose capacity depends on its
//! escape analysis; the evaluator always takes the heap's.

/// How Go lays out a value of a type: its size and alignment in bytes and whether it holds
/// pointers (a slice of it is then scanned by the collector, which changes its size class).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Layout {
    pub size: u64,
    pub align: u64,
    pub pointers: bool,
}

impl Layout {
    pub(crate) const fn scalar(size: u64) -> Layout {
        Layout {
            size,
            align: size,
            pointers: false,
        }
    }

    /// `n` words, the first a pointer: a string (2), a slice (3), an interface (2), a pointer,
    /// map, channel or function (1).
    pub(crate) const fn words(n: u64) -> Layout {
        Layout {
            size: 8 * n,
            align: 8,
            pointers: true,
        }
    }

    /// A struct of `fields` in order, padded as Go pads it: each field at its alignment, a final
    /// zero-size field given one byte, the whole rounded up to the largest alignment.
    pub(crate) fn record(fields: &[Layout]) -> Layout {
        let mut size = 0u64;
        let mut align = 1u64;
        let mut pointers = false;
        for f in fields {
            size = size.div_ceil(f.align.max(1)) * f.align.max(1);
            size = size.saturating_add(f.size);
            align = align.max(f.align);
            pointers |= f.pointers;
        }
        if fields.last().is_some_and(|f| f.size == 0) && size > 0 {
            size += 1;
        }
        Layout {
            size: size.div_ceil(align) * align,
            align,
            pointers,
        }
    }

    pub(crate) fn array(len: u64, elem: Layout) -> Layout {
        Layout {
            size: elem.size.saturating_mul(len),
            align: elem.align,
            pointers: elem.pointers && len > 0,
        }
    }
}

const SIZE_CLASSES: [u64; 68] = [
    0, 8, 16, 24, 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192, 208, 224, 240, 256, 288, 320,
    352, 384, 416, 448, 480, 512, 576, 640, 704, 768, 896, 1024, 1152, 1280, 1408, 1536, 1792,
    2048, 2304, 2688, 3072, 3200, 3456, 4096, 4864, 5376, 6144, 6528, 6784, 6912, 8192, 9472, 9728,
    10240, 10880, 12288, 13568, 14336, 16384, 18432, 19072, 20480, 21760, 24576, 27264, 28672,
    32768,
];
const MAX_SMALL_SIZE: u64 = 32768;
const MALLOC_HEADER_SIZE: u64 = 8;
const MIN_SIZE_FOR_MALLOC_HEADER: u64 = 8 * 64;
const PAGE_SIZE: u64 = 8192;

/// `runtime.roundupsize`: the bytes the allocator gives a request of `size`.
fn roundupsize(size: u64, noscan: bool) -> u64 {
    if size <= MAX_SMALL_SIZE - MALLOC_HEADER_SIZE {
        let mut req = size;
        if !noscan && req > MIN_SIZE_FOR_MALLOC_HEADER {
            req += MALLOC_HEADER_SIZE;
        }
        let class = SIZE_CLASSES
            .iter()
            .copied()
            .find(|&c| c >= req)
            .unwrap_or(MAX_SMALL_SIZE);
        return class - (req - size);
    }
    match size.checked_add(PAGE_SIZE - 1) {
        Some(r) => r & !(PAGE_SIZE - 1),
        None => size,
    }
}

/// `runtime.nextslicecap`.
fn nextslicecap(new_len: u64, old_cap: u64) -> u64 {
    let double = old_cap.saturating_mul(2);
    if new_len > double {
        return new_len;
    }
    const THRESHOLD: u64 = 256;
    if old_cap < THRESHOLD {
        return double;
    }
    let mut cap = old_cap;
    while cap < new_len {
        cap = cap.saturating_add((cap + 3 * THRESHOLD) >> 2);
    }
    cap
}

/// The capacity `runtime.growslice` gives a slice of elements of `size` bytes, holding pointers
/// when `pointers`, grown from capacity `old_cap` to hold `new_len` elements.
pub(crate) fn grow_cap(new_len: usize, old_cap: usize, size: u64, pointers: bool) -> usize {
    if size == 0 {
        return new_len;
    }
    let cap = nextslicecap(new_len as u64, old_cap as u64);
    let mem = roundupsize(cap.saturating_mul(size), !pointers);
    usize::try_from(mem / size)
        .unwrap_or(usize::MAX)
        .max(new_len)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Capacities go1.27.1 prints for `cap` after appending one element at a time to a nil
    /// slice of int, string and byte that escapes to the heap, and for one append of five ints
    /// to nil.
    #[test]
    fn capacities_are_gos() {
        let ladder = |size, pointers, n| {
            let mut caps = Vec::new();
            let mut cap = 0;
            for len in 1..=n {
                if len > cap {
                    cap = grow_cap(len, cap, size, pointers);
                    caps.push(cap);
                }
            }
            caps
        };
        assert_eq!(
            ladder(8, false, 600),
            [1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 848]
        );
        assert_eq!(ladder(16, true, 40), [1, 2, 4, 8, 16, 32, 71]);
        assert_eq!(ladder(1, false, 40), [8, 16, 32, 64]);
        assert_eq!(grow_cap(5, 0, 8, false), 6);
    }

    #[test]
    fn a_struct_is_padded_as_go_pads_it() {
        let b = Layout::scalar(1);
        let i = Layout::scalar(8);
        assert_eq!(Layout::record(&[b, i, b]).size, 24);
        assert_eq!(Layout::record(&[i, Layout::record(&[])]).size, 16);
        assert_eq!(Layout::record(&[]).size, 0);
    }
}
