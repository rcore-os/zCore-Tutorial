//! Objects for Virtual Memory Management.

mod vmar;
mod vmo;

pub use self::{vmar::*, vmo::*};

/// Physical Address
pub type PhysAddr = usize;

/// Virtual Address
pub type VirtAddr = usize;

/// Device Address
pub type DevVAddr = usize;

/// Size of a page
pub const PAGE_SIZE: usize = 0x1000;

/// log2(PAGE_SIZE)
pub const PAGE_SIZE_LOG2: usize = 12;

/// Check whether `x` is a multiple of `PAGE_SIZE`.
pub fn page_aligned(x: usize) -> bool {
    check_aligned(x, PAGE_SIZE)
}

/// Check whether `x` is a multiple of `align`.
pub fn check_aligned(x: usize, align: usize) -> bool {
    x % align == 0
}

/// How many pages the `size` needs.
/// To avoid overflow and pass more unit tests, use wrapping add
pub fn pages(size: usize) -> usize {
    ceil(size, PAGE_SIZE)
}

/// How many `align` the `x` needs.
pub fn ceil(x: usize, align: usize) -> usize {
    x.wrapping_add(align - 1) / align
}

/// Round up `size` to a multiple of `PAGE_SIZE`.
pub fn roundup_pages(size: usize) -> usize {
    pages(size) * PAGE_SIZE
}

/// Round down `size` to a multiple of `PAGE_SIZE`.
pub fn round_down_pages(size: usize) -> usize {
    size / PAGE_SIZE * PAGE_SIZE
}
