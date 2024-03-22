use crate::memory::AreaFrameAllocator;

pub mod display;
pub mod keyboard;
pub mod pci;
pub mod storage;
pub mod timer;

pub fn init(frame_allocator: &mut AreaFrameAllocator) {
    pci::init();
    keyboard::init();
    storage::init(frame_allocator);
}
