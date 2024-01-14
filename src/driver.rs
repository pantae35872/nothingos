pub mod keyboard;
pub mod pci;
pub mod storage;
pub mod timer;

pub fn init() {
    keyboard::init();
}
