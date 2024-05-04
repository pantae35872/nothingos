use crate::memory::AreaFrameAllocator;

pub mod display;
pub mod pci;
pub mod storage;

pub fn init(are_frame_allocator: &mut AreaFrameAllocator) {
    pci::init();
    storage::init(are_frame_allocator);
}
