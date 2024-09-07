#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(pointer_is_aligned_to)]
#![reexport_test_harness_main = "test_main"]
#![test_runner(nothingos::test_runner)]

extern crate alloc;
extern crate nothingos;

use core::alloc::Layout;

use alloc::{alloc::alloc, vec::Vec};
use common::boot::BootInformation;
use nothingos::memory::allocator::buddy_allocator::BuddyAllocator;

#[no_mangle]
pub extern "C" fn start(multiboot_information_address: *mut BootInformation) -> ! {
    nothingos::init(multiboot_information_address);
    test_main();
    loop {}
}

#[test_case]
fn simple_alloc() {
    let buf = unsafe { alloc(Layout::from_size_align(256, 256).unwrap()) };
    let mut heap = unsafe { BuddyAllocator::<64>::new(buf as usize, 256) };
    let sizes = [16, 32, 16, 32, 8, 8, 16, 128];
    let mut allocations = Vec::new();
    let mut allocation_ranges = Vec::new();

    for &size in sizes.iter() {
        let ptr = heap.allocate(size);
        assert!(ptr.is_some(), "Allocation failed for size: {}", size);
        let ptr = ptr.unwrap();
        assert!(ptr.is_aligned_to(size));

        let start = ptr.as_ptr() as usize;
        let end = start + size - 1;

        allocation_ranges.push((start, end));
        allocations.push((ptr, size));
    }

    for i in 0..allocation_ranges.len() {
        for j in i + 1..allocation_ranges.len() {
            let (start_i, end_i) = allocation_ranges[i];
            let (start_j, end_j) = allocation_ranges[j];

            assert!(
                end_i < start_j || end_j < start_i,
                "Memory overlap detected between allocations: ({}, {}) and ({}, {})",
                start_i,
                end_i,
                start_j,
                end_j
            );
        }
    }
}
