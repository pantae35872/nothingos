use crate::MemoryController;

pub mod display;
pub mod keyboard;
pub mod pci;
pub mod storage;
pub mod timer;

pub fn init(memory_controller: &mut MemoryController) {
    pci::init();
    keyboard::init();
    storage::init(memory_controller);
}
