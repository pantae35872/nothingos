use core::arch::asm;

use crate::gdt;
use crate::hlt_loop;
use crate::memory::paging::Page;
use crate::memory::Frame;
use crate::print;
use crate::println;
use crate::EntryFlags;
use crate::MemoryController;
use alloc::ffi::CString;
use conquer_once::spin::OnceCell;
use lazy_static::lazy_static;
use spin::Mutex;
use x2apic::lapic::xapic_base;
use x2apic::lapic::LocalApic;
use x2apic::lapic::LocalApicBuilder;
use x2apic::lapic::TimerMode;
use x86_64::structures::idt::PageFaultErrorCode;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use x86_64::VirtAddr;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handle);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.overflow.set_handler_fn(overflow_handler);
        idt.divide_error.set_handler_fn(divide_handler);
        idt.debug.set_handler_fn(debug_handler);
        idt.invalid_tss.set_handler_fn(tss_handler);
        idt.machine_check.set_handler_fn(machine_check_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.hv_injection_exception
            .set_handler_fn(hv_injection_handler);
        idt.device_not_available
            .set_handler_fn(device_not_available_handler);
        idt.vmm_communication_exception
            .set_handler_fn(vmm_communication_exception_handler);
        idt.virtualization.set_handler_fn(virtualization_handler);
        idt.security_exception
            .set_handler_fn(security_exception_handler);
        idt.alignment_check.set_handler_fn(alignment_check_handler);
        idt.x87_floating_point
            .set_handler_fn(x87_floating_point_handler);
        idt.segment_not_present
            .set_handler_fn(segment_not_present_handler);
        idt.general_protection_fault
            .set_handler_fn(general_protection_fault_handler);
        idt.cp_protection_exception
            .set_handler_fn(cp_protection_exception_handler);
        idt.stack_segment_fault
            .set_handler_fn(stack_segment_fault_handler);
        idt.simd_floating_point
            .set_handler_fn(simd_floating_point_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt[InterruptIndex::PrimaryATA.as_usize()].set_handler_fn(primary_ata_interrupt_handler);
        idt[InterruptIndex::SecondaryATA.as_usize()]
            .set_handler_fn(secondary_ata_interrupt_handler);
        unsafe {
            idt[0x80].set_handler_addr(VirtAddr::new(syscall as u64));
        }
        idt
    };
}

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub const LAPIC_VADDR: usize = 0xFFFFFFFFFFF00000;
pub const LAPIC_SIZE: usize = 0xFFF;
pub static LAPICS: OnceCell<Mutex<LocalApic>> = OnceCell::uninit();

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
    PrimaryATA = PIC_1_OFFSET + 14,
    SecondaryATA = PIC_1_OFFSET + 15,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

pub fn init(memory_controller: &mut MemoryController) {
    let apic_physical_address: u64 = unsafe { xapic_base() };
    let apic_start_page = Page::containing_address(LAPIC_VADDR);
    let apic_end_page = Page::containing_address(LAPIC_VADDR + LAPIC_SIZE - 1);
    for (page, frame) in
        Page::range_inclusive(apic_start_page, apic_end_page).zip(Frame::range_inclusive(
            Frame::containing_address(apic_physical_address as usize),
            Frame::containing_address(apic_physical_address as usize + LAPIC_SIZE - 1),
        ))
    {
        memory_controller.active_table.map_to(
            page,
            frame,
            EntryFlags::PRESENT
                | EntryFlags::NO_CACHE
                | EntryFlags::WRITABLE
                | EntryFlags::WRITE_THROUGH,
            memory_controller.frame_allocator,
        );
    }
    LAPICS.init_once(|| {
        let mut lapic = LocalApicBuilder::new()
            .timer_vector(32)
            .error_vector(34)
            .spurious_vector(33)
            .set_xapic_base(LAPIC_VADDR as u64)
            .build()
            .expect("Could not create lapic");
        unsafe {
            lapic.enable();
        }
        Mutex::new(lapic)
    });
    IDT.load();
}

extern "x86-interrupt" fn simd_floating_point_handler(_stack_frame: InterruptStackFrame) {}

extern "x86-interrupt" fn x87_floating_point_handler(_stack_frame: InterruptStackFrame) {}

extern "x86-interrupt" fn virtualization_handler(_stack_frame: InterruptStackFrame) {}

extern "x86-interrupt" fn device_not_available_handler(_stack_frame: InterruptStackFrame) {}

extern "x86-interrupt" fn hv_injection_handler(_stack_frame: InterruptStackFrame) {}

extern "x86-interrupt" fn invalid_opcode_handler(_stack_frame: InterruptStackFrame) {}

extern "x86-interrupt" fn machine_check_handler(_stack_frame: InterruptStackFrame) -> ! {
    hlt_loop();
}

extern "x86-interrupt" fn stack_segment_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) {
}

extern "x86-interrupt" fn cp_protection_exception_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) {
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: GENERAL PROTECTION FAULT\n{:#?}, ERROR_CODE: {}",
        stack_frame, error_code
    );
}

extern "x86-interrupt" fn segment_not_present_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) {
}

extern "x86-interrupt" fn alignment_check_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) {
}

extern "x86-interrupt" fn security_exception_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) {
}

extern "x86-interrupt" fn vmm_communication_exception_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) {
}

extern "x86-interrupt" fn tss_handler(_stack_frame: InterruptStackFrame, _error_code: u64) {}

extern "x86-interrupt" fn debug_handler(_stack_frame: InterruptStackFrame) {}

extern "x86-interrupt" fn divide_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: DIVISION\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: OVERFLOW\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn breakpoint_handle(_stack_frame: InterruptStackFrame) {
    println!("BreakPoint");
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        LAPICS.get().unwrap().lock().end_of_interrupt();
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    println!("AAA");
    /*let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    crate::driver::keyboard::keyboard_scancode(scancode);*/
    unsafe {
        LAPICS.get().unwrap().lock().end_of_interrupt();
    }
    /*unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }*/
}

#[naked]
#[no_mangle]
fn syscall() {
    unsafe {
        asm!(
            r#"
        push r11
        push r10
        push r9
        push r8
        push rdi
        push rsi
        push rdx
        push rcx
        push rax
        
        mov rdi, rsp
        call inner_syscall
        pop rax
        pop rcx
        pop rdx
        pop rsi
        pop rdi
        pop r8
        pop r9
        pop r10
        pop r11
        iretq
        "#,
            options(noreturn)
        );
    }
}

#[derive(Debug)]
#[repr(C)]
struct FullInterruptStackFrame {
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub instruction_pointer: VirtAddr,
    pub code_segment: u64,
    pub cpu_flags: u64,
    pub stack_pointer: VirtAddr,
    pub stack_segment: u64,
}
#[no_mangle]
extern "C" fn inner_syscall(stack_frame: &mut FullInterruptStackFrame) {
    if stack_frame.rax == 1 {
        let data = unsafe { CString::from_raw(stack_frame.rcx as *mut i8) };
        print!("{}", data.to_str().unwrap());
    }
}

extern "x86-interrupt" fn primary_ata_interrupt_handler(_stack_frame: InterruptStackFrame) {
    //println!("a");

    unsafe {
        LAPICS.get().unwrap().lock().end_of_interrupt();
    }
    /*unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::PrimaryATA.as_u8());
    }*/
}

extern "x86-interrupt" fn secondary_ata_interrupt_handler(_stack_frame: InterruptStackFrame) {
    //println!("aa");

    unsafe {
        LAPICS.get().unwrap().lock().end_of_interrupt();
    }

    /*unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::SecondaryATA.as_u8());
    }*/
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    panic!("PAGE FAULT");
}
