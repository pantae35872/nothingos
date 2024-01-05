use core::ptr;
use core::ptr::write_volatile;

use lazy_static::lazy_static;

pub const BACKBUFFER_START: usize = 0o_000_100_000_000_0000;
pub const BACKBUFFER_SIZE: usize = 0xFA000;

use crate::utils::port::Port8Bit;

pub struct Vga {
    misc_port: Port8Bit,
    crtc_index_port: Port8Bit,
    crtc_data_port: Port8Bit,
    sequencer_index_port: Port8Bit,
    sequencer_data_port: Port8Bit,
    graphic_controller_index_port: Port8Bit,
    graphic_controller_data_port: Port8Bit,
    attribute_controller_index_port: Port8Bit,
    attribute_controller_write_port: Port8Bit,
    attribute_controller_reset_port: Port8Bit,

}

lazy_static! {
    pub static ref VGA: Vga = Vga::new();
}

impl Vga {
    pub fn new() -> Self {
        Self {
            misc_port: Port8Bit::new(0x3c2),
            crtc_index_port: Port8Bit::new(0x3d4),
            crtc_data_port: Port8Bit::new(0x3d5),
            sequencer_index_port: Port8Bit::new(0x3c4),
            sequencer_data_port: Port8Bit::new(0x3c5),
            graphic_controller_index_port: Port8Bit::new(0x3ce),
            graphic_controller_data_port: Port8Bit::new(0x3cf),
            attribute_controller_index_port: Port8Bit::new(0x3c0),
            attribute_controller_write_port: Port8Bit::new(0x3c0),
            attribute_controller_reset_port: Port8Bit::new(0x3da),
        }
    }

    unsafe fn write_registers(&self, registers_slice: &mut [u8]) {
        let mut registers = registers_slice.iter();
        self.misc_port.write(*registers.next().unwrap_or(&0));
        for i in 0..5 {
            self.sequencer_index_port.write(i);
            self.sequencer_data_port
                .write(*registers.next().unwrap_or(&0));
        }

        self.crtc_index_port.write(0x03);
        self.crtc_data_port.write(self.crtc_data_port.read() | 0x80);
        self.crtc_index_port.write(0x11);
        self.crtc_data_port
            .write(self.crtc_data_port.read() & !0x80);
        drop(registers);
        registers_slice[0x03] = registers_slice[0x03] | 0x80;
        registers_slice[0x11] = registers_slice[0x11] & !0x80;
        let mut registers = registers_slice.iter().skip(6);

        for i in 0..25 {
            self.crtc_index_port.write(i);
            self.crtc_data_port.write(*registers.next().unwrap_or(&0));
        }

        for i in 0..9 {
            self.graphic_controller_index_port.write(i);
            self.graphic_controller_data_port
                .write(*registers.next().unwrap_or(&0));
        }

        for i in 0..21 {
            self.attribute_controller_reset_port.read();
            self.attribute_controller_index_port.write(i);
            self.attribute_controller_write_port
                .write(*registers.next().unwrap_or(&0));
        }

        self.attribute_controller_reset_port.read();
        self.attribute_controller_index_port.write(0x20);
    }

    fn support_mode(width: u32, height: u32, colordepth: u32) -> bool {
        width == 320 && height == 200 && colordepth == 8
    }

    pub fn set_mode(&self, width: u32, height: u32, colordepth: u32) -> bool {
        if !Vga::support_mode(width, height, colordepth) {
            return false;
        }

        let mut g_320x200x256: [u8; 61] = [
            /* MISC */
            0x63, /* SEQ */
            0x03, 0x01, 0x0F, 0x00, 0x0E, /* CRTC */
            0x5F, 0x4F, 0x50, 0x82, 0x54, 0x80, 0xBF, 0x1F, 0x00, 0x41, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x9C, 0x0E, 0x8F, 0x28, 0x40, 0x96, 0xB9, 0xA3, 0xFF, /* GC */
            0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x05, 0x0F, 0xFF, /* AC */
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
            0x0E, 0x0F, 0x41, 0x00, 0x0F, 0x00, 0x00,
        ];

        unsafe {
            self.write_registers(&mut g_320x200x256);
        }
        return true;
    }

    fn get_frame_buffer_segment(&self) -> usize {
        self.graphic_controller_index_port.write(0x06);
        let segment_number = self.graphic_controller_data_port.read() & (3 << 2);
        match segment_number {
            n if 0 << 2 == n => 0x00000,
            n if 1 << 2 == n => BACKBUFFER_START,
            n if 2 << 2 == n => 0xB0000,
            n if 3 << 2 == n => 0xB8000,
            _ => 0,
        }
    }

    pub fn put_pixel(&self, x: usize, y: usize, color_index: u8) {
        if 320 <= x || 200 <= y {
            return;
        }
        unsafe {
            write_volatile(
                (self.get_frame_buffer_segment() + 320 * y + x) as *mut u8,
                color_index,
            );
        }
    }

    pub fn swap(&self) {
        unsafe {
            ptr::copy_nonoverlapping(BACKBUFFER_START as *const u8, 0xA0000 as *mut u8, 0xFA00);
        }
    }

    pub fn clear(&self) {
        unsafe {
            ptr::write_bytes(BACKBUFFER_START as *mut u8, 0, 0xFA000);
        }
    }
}
