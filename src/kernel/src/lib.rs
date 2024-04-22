#![no_std]
#![feature(pointer_is_aligned)]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]
#![feature(ptr_internals)]
#![feature(const_mut_refs)]
#![feature(str_from_utf16_endian)]
#![allow(internal_features)]
#![allow(dead_code)]
#[macro_use]
extern crate bitflags;

bitflags! {
    #[derive(Clone, Copy)]
    pub struct EntryFlags: u64 {
        const PRESENT =         1 << 0;
        const WRITABLE =        1 << 1;
        const USER_ACCESSIBLE = 1 << 2;
        const WRITE_THROUGH =   1 << 3;
        const NO_CACHE =        1 << 4;
        const ACCESSED =        1 << 5;
        const DIRTY =           1 << 6;
        const HUGE_PAGE =       1 << 7;
        const GLOBAL =          1 << 8;
        const NO_EXECUTE =      1 << 63;
    }
}

impl EntryFlags {
    pub fn from_elf_section_flags(section: &SectionHeaderEntry) -> EntryFlags {
        let mut flags = EntryFlags::empty();

        if section.flags().contains(SectionHeaderFlags::SHF_ALLOC) {
            // section is loaded to memory
            flags = flags | EntryFlags::PRESENT;
        }
        if section.flags().contains(SectionHeaderFlags::SHF_WRITE) {
            flags = flags | EntryFlags::WRITABLE;
        }
        if !section.flags().contains(SectionHeaderFlags::SHF_EXECINSTR) {
            flags = flags | EntryFlags::NO_EXECUTE;
        }

        flags
    }
}

impl Display for EntryFlags {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "flag: {}", self.0)
    }
}

extern crate alloc;
extern crate core;
extern crate lazy_static;
extern crate spin;

pub mod allocator;
pub mod driver;
pub mod filesystem;
pub mod gdt;
pub mod interrupt;
pub mod memory;
pub mod print;
pub mod serial;
pub mod task;
pub mod utils;

use core::fmt::Display;
use core::panic::PanicInfo;
use core::usize;

use allocator::{HEAP_SIZE, HEAP_START};
use conquer_once::spin::OnceCell;
use elf_rs::{Elf, SectionHeaderEntry, SectionHeaderFlags};
use interrupt::LAPIC_SIZE;
use memory::paging::{ActivePageTable, Page};
use memory::stack_allocator::{Stack, StackAllocator};
use memory::AreaFrameAllocator;
use spin::Mutex;
use uefi::proto::console::gop::Mode;
use uefi::table::boot::MemoryMap;
use x2apic::lapic::xapic_base;
use x86_64::registers::control::Cr0Flags;
use x86_64::registers::model_specific::EferFlags;
use x86_64::{PhysAddr, VirtAddr};

pub trait Testable {
    fn run(&self) -> ();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    //println!("{}", info);
    #[cfg(feature = "test")]
    {
        test_panic_handler(info);
    }
    serial_println!("panic! {}", info);
    #[cfg(not(feature = "test"))]
    hlt_loop();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        #[cfg(feature = "test")]
        serial_print!("{}...\t", core::any::type_name::<T>());
        #[cfg(feature = "test")]
        self();
        #[cfg(feature = "test")]
        serial_println!("[ok]");
    }
}

pub fn test_runner(_tests: &[&dyn Testable]) {
    #[cfg(feature = "test")]
    serial_println!("Running {} tests", _tests.len());
    #[cfg(feature = "test")]
    for test in _tests {
        test.run();
    }
    #[cfg(feature = "test")]
    exit_qemu(QemuExitCode::Success);
}

#[cfg(feature = "test")]
pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

static ACTIVE_TABLE: OnceCell<Mutex<ActivePageTable>> = OnceCell::uninit();
pub fn get_physical(address: VirtAddr) -> Option<PhysAddr> {
    match ACTIVE_TABLE.get() {
        Some(memory_controller) => {
            match memory_controller
                .lock()
                .translate(address.as_u64() as usize)
            {
                Some(addr) => return Some(PhysAddr::new(addr as u64)),
                None => return None,
            };
        }
        None => return None,
    }
}

pub fn get_physical_with_controller(
    address: VirtAddr,
    memory_controller: &mut MemoryController,
) -> Option<PhysAddr> {
    match memory_controller
        .active_table
        .translate(address.as_u64() as usize)
    {
        Some(addr) => return Some(PhysAddr::new(addr as u64)),
        None => return None,
    };
}

#[repr(C)]
#[derive(Debug)]
pub struct BootInformation {
    pub largest_addr: u64,
    pub gop_mode: Mode,
    pub framebuffer: *mut u32,
    pub runtime_system_table: u64,
    pub memory_map: *mut MemoryMap<'static>,
    pub kernel_start: u64,
    pub kernel_end: u64,
    pub elf_section: Elf<'static>,
    pub boot_info_start: u64,
    pub boot_info_end: u64,
}
pub struct MemoryController<'area_frame_allocator, 'active_table> {
    active_table: &'active_table mut ActivePageTable,
    frame_allocator: AreaFrameAllocator<'area_frame_allocator>,
    stack_allocator: StackAllocator,
}

impl<'area_frame_allocator, 'active_table> MemoryController<'area_frame_allocator, 'active_table> {
    pub fn alloc_stack(&mut self, size_in_pages: usize) -> Option<Stack> {
        let &mut MemoryController {
            ref mut active_table,
            ref mut frame_allocator,
            ref mut stack_allocator,
        } = self;
        stack_allocator.alloc_stack(active_table, frame_allocator, size_in_pages)
    }

    pub fn get_physical(&mut self, addr: VirtAddr) -> Option<PhysAddr> {
        match self.active_table.translate(addr.as_u64() as usize) {
            Some(addr) => return Some(PhysAddr::new(addr as u64)),
            None => return None,
        };
    }
}
pub fn init(information_address: *mut BootInformation) {
    let boot_info = unsafe { &mut *information_address };
    let memory_map = unsafe { &*boot_info.memory_map };
    let apic_physical_address: u64 = unsafe { xapic_base() };
    let mut frame_allocator = memory::area_frame_allocator::AreaFrameAllocator::new(
        boot_info.kernel_start as usize,
        boot_info.kernel_end as usize,
        apic_physical_address as usize,
        apic_physical_address as usize + LAPIC_SIZE as usize - 1,
        memory_map,
    );
    enable_nxe_bit();
    enable_write_protect_bit();
    let mut active_table = memory::remap_the_kernel(&mut frame_allocator, &boot_info);
    let stack_allocator = {
        let stack_alloc_start = Page::containing_address(HEAP_START + HEAP_SIZE);
        let stack_alloc_end = stack_alloc_start + 100;
        let stack_alloc_range = Page::range_inclusive(stack_alloc_start, stack_alloc_end);
        StackAllocator::new(stack_alloc_range)
    };
    let mut memory_controller = MemoryController {
        active_table: &mut active_table,
        frame_allocator,
        stack_allocator,
    };
    gdt::init_gdt(&mut memory_controller);
    interrupt::init(&mut memory_controller);
    x86_64::instructions::interrupts::enable();
    print::init(0xb8000, 0xb, 0);
    allocator::init(&mut memory_controller);
    driver::init(&mut memory_controller);
    task::init();
    ACTIVE_TABLE.init_once(|| Mutex::new(active_table));
}

fn enable_write_protect_bit() {
    use x86_64::registers::control::Cr0;

    unsafe {
        let mut cr0 = Cr0::read();
        cr0.insert(Cr0Flags::WRITE_PROTECT);
        Cr0::write(cr0);
    }
}

fn enable_nxe_bit() {
    use x86_64::registers::model_specific::Efer;

    unsafe {
        let mut efer = Efer::read();
        efer.insert(EferFlags::NO_EXECUTE_ENABLE);
        Efer::write(efer);
    }
}

#[cfg(feature = "test")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

#[cfg(feature = "test")]
pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}
