use core::mem::size_of;
use core::{ptr, u8};

use alloc::alloc::*;
use x86_64::VirtAddr;
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::{Mapper, Size4KiB, FrameAllocator, Page, PageTableFlags};

pub const HEAP_START: usize = 0o_000_001_000_000_0000;
pub const HEAP_SIZE: usize = 1000 * 1024; // 100 KiB

struct Metadata {
    start: u8,
    size: usize,
    end: u8
}

pub struct Allocator {
    start: usize,
    end: usize,
}

impl Allocator {
    const fn new(start: usize, size: usize) -> Self {
        Allocator { start, end: start + size }
    }

    unsafe fn is_valid_metadata_ptr(ptr: *const Metadata) -> bool {
        !ptr.is_null() && (*ptr).start == 0xAA && (*ptr).end == 0xBB
    }

    unsafe fn is_valid_dealloc_metadata_ptr(ptr: *const Metadata) -> bool {
        !ptr.is_null() && (*ptr).start == 0xCC && (*ptr).end == 0xDD
    }

    unsafe fn allocate(&self, size: usize, align: usize) -> Option<*mut u8> {
        let mut current_pos = self.start;
        let metadata_size = size_of::<Metadata>();
        let mut align_posistion = {
            let align_mask = align - 1;
            let unaligned_position = (current_pos + align_mask) & !align_mask;
            unaligned_position + metadata_size
        };
        while Self::is_valid_metadata_ptr(align_posistion as *const Metadata) || Self::is_valid_dealloc_metadata_ptr(align_posistion as *const Metadata) {
            if Self::is_valid_dealloc_metadata_ptr(align_posistion as *const Metadata) {
                if (*(align_posistion as *const Metadata)).size == size {
                    break; 
                } else if (*(align_posistion as *const Metadata)).size >= (size + metadata_size) {
                    let metadata_pos = {
                        let align_mask = align - 1;
                        let unaligned_position = ((current_pos + size) + align_mask) & !align_mask;
                        unaligned_position + metadata_size * 2             
                    };
                    (*(metadata_pos as *mut Metadata)).size = ((*(align_posistion as *const Metadata)).size) - (size + metadata_size);
                    (*(metadata_pos as *mut Metadata)).start = 0xCC;
                    (*(metadata_pos as *mut Metadata)).end = 0xDD;
                    break;
                }
            }
            current_pos += metadata_size + (*(align_posistion as *const Metadata)).size;
            align_posistion = {
                let align_mask = align - 1;
                let unaligned_position = (current_pos + align_mask) & !align_mask;
                unaligned_position + metadata_size
            };
        }

        let total_size = align_posistion - current_pos + metadata_size + size;
        
        if total_size <= (self.end - current_pos) {
            let metadata_ptr = align_posistion as *mut Metadata;
            (*(align_posistion as *mut Metadata)).size = size;
            (*(align_posistion as *mut Metadata)).start = 0xAA;
            (*(align_posistion as *mut Metadata)).end = 0xBB;

            Some((metadata_ptr as usize + metadata_size) as *mut u8)
        } else {
            None
        }
    }
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match self.allocate(layout.size(), layout.align()) {
            Some(ptr) => ptr,
            None => ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
       if !ptr.is_null() {
           let metadata_ptr = (ptr as usize - size_of::<Metadata>()) as *mut Metadata;
           if Self::is_valid_metadata_ptr(metadata_ptr) && (*metadata_ptr).size == layout.size() {
               (*metadata_ptr).start = 0xCC;
               (*metadata_ptr).end = 0xDD;
           }
       }
    }
}

pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush()
        };
    }

    Ok(())
}

#[global_allocator]
static GLOBAL_ALLOCATOR: Allocator = Allocator::new(HEAP_START, HEAP_SIZE);
