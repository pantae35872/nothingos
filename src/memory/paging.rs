pub use self::entry::*;
use self::mapper::Mapper;
use self::table::{Level4, Table};
use self::temporary_page::TemporaryPage;
use crate::memory::{Frame, FrameAllocator, PAGE_SIZE};
use crate::EntryFlags;
use core::ops::{Deref, DerefMut};
use core::ptr::Unique;
use multiboot2::BootInformation;
use x86_64::registers::control::{self, Cr3, Cr3Flags};
use x86_64::structures::paging::PhysFrame;
use x86_64::{PhysAddr, VirtAddr};

mod entry;
mod mapper;
mod table;
mod temporary_page;

const ENTRY_COUNT: usize = 512;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Page {
    number: usize,
}

impl Page {
    pub fn containing_address(address: VirtualAddress) -> Page {
        assert!(
            address < 0x0000_8000_0000_0000 || address >= 0xffff_8000_0000_0000,
            "invalid address: 0x{:x}",
            address
        );
        Page {
            number: address / PAGE_SIZE,
        }
    }

    fn start_address(&self) -> usize {
        self.number * PAGE_SIZE
    }

    fn p4_index(&self) -> usize {
        (self.number >> 27) & 0o777
    }
    fn p3_index(&self) -> usize {
        (self.number >> 18) & 0o777
    }
    fn p2_index(&self) -> usize {
        (self.number >> 9) & 0o777
    }
    fn p1_index(&self) -> usize {
        (self.number >> 0) & 0o777
    }

    pub fn range_inclusive(start: Page, end: Page) -> PageIter {
        PageIter { start, end }
    }
}

pub struct PageIter {
    start: Page,
    end: Page,
}

impl Iterator for PageIter {
    type Item = Page;

    fn next(&mut self) -> Option<Page> {
        if self.start <= self.end {
            let page = self.start;
            self.start.number += 1;
            Some(page)
        } else {
            None
        }
    }
}

pub struct ActivePageTable {
    p4: Unique<Table<Level4>>,
    mapper: Mapper,
}

impl Deref for ActivePageTable {
    type Target = Mapper;

    fn deref(&self) -> &Mapper {
        &self.mapper
    }
}

impl DerefMut for ActivePageTable {
    fn deref_mut(&mut self) -> &mut Mapper {
        &mut self.mapper
    }
}

impl ActivePageTable {
    unsafe fn new() -> ActivePageTable {
        ActivePageTable {
            p4: Unique::new_unchecked(table::P4),
            mapper: Mapper::new(),
        }
    }

    fn p4_mut(&mut self) -> &mut Table<Level4> {
        unsafe { self.p4.as_mut() }
    }

    pub fn with<F>(
        &mut self,
        table: &mut InactivePageTable,
        temporary_page: &mut temporary_page::TemporaryPage, 
        f: F,
    ) where
        F: FnOnce(&mut Mapper),
    {
        use x86_64::instructions::tlb;
        {
            let (level_4_table_frame, _) = control::Cr3::read();
            let backup =
                Frame::containing_address(level_4_table_frame.start_address().as_u64() as usize);

            let p4_table = temporary_page.map_table_frame(backup.clone(), self);

            self.p4_mut()[511].set(
                table.p4_frame.clone(),
                EntryFlags::PRESENT | EntryFlags::WRITABLE,
            );
            tlb::flush_all();

            f(self);

            p4_table[511].set(backup, EntryFlags::PRESENT | EntryFlags::WRITABLE);
            tlb::flush_all();
        }

        temporary_page.unmap(self);
    }

    pub fn switch(&mut self, new_table: InactivePageTable) -> InactivePageTable {
        let (level_4_table_frame, _) = control::Cr3::read();
        let old_table = InactivePageTable {
            p4_frame: Frame::containing_address(
                level_4_table_frame.start_address().as_u64() as usize
            ),
        };
        let frame =
            PhysFrame::from_start_address(PhysAddr::new(new_table.p4_frame.start_address() as u64));
        unsafe {
            Cr3::write(
                frame.expect("Failed to cr3 Frame"),
                Cr3Flags::PAGE_LEVEL_WRITETHROUGH,
            );
        }
        old_table
    }

    fn p4(&self) -> &Table<Level4> {
        unsafe { self.p4.as_ref() }
    }

    pub fn translate(&self, virtual_address: VirtualAddress) -> Option<PhysicalAddress> {
        let offset = virtual_address % PAGE_SIZE;
        self.translate_page(Page::containing_address(virtual_address))
            .map(|frame| frame.number * PAGE_SIZE + offset)
    }

    fn translate_page(&self, page: Page) -> Option<Frame> {
        let p3 = self.p4().next_table(page.p4_index());

        let huge_page = || {
            p3.and_then(|p3| {
                let p3_entry = &p3[page.p3_index()];
                if let Some(start_frame) = p3_entry.pointed_frame() {
                    if p3_entry.flags().contains(EntryFlags::HUGE_PAGE) {
                        assert!(start_frame.number % (ENTRY_COUNT * ENTRY_COUNT) == 0);
                        return Some(Frame {
                            number: start_frame.number
                                + page.p2_index() * ENTRY_COUNT
                                + page.p1_index(),
                        });
                    }
                }
                if let Some(p2) = p3.next_table(page.p3_index()) {
                    let p2_entry = &p2[page.p2_index()];
                    if let Some(start_frame) = p2_entry.pointed_frame() {
                        if p2_entry.flags().contains(EntryFlags::HUGE_PAGE) {
                            assert!(start_frame.number % ENTRY_COUNT == 0);
                            return Some(Frame {
                                number: start_frame.number + page.p1_index(),
                            });
                        }
                    }
                }
                None
            })
        };

        p3.and_then(|p3| p3.next_table(page.p3_index()))
            .and_then(|p2| p2.next_table(page.p2_index()))
            .and_then(|p1| p1[page.p1_index()].pointed_frame())
            .or_else(huge_page)
    }

    fn unmap<A>(&mut self, page: Page, allocator: &mut A)
    where
        A: FrameAllocator,
    {
        use x86_64::instructions::tlb;

        assert!(self.translate(page.start_address()).is_some());

        let p1 = self
            .p4_mut()
            .next_table_mut(page.p4_index())
            .and_then(|p3| p3.next_table_mut(page.p3_index()))
            .and_then(|p2| p2.next_table_mut(page.p2_index()))
            .expect("mapping code does not support huge pages");
        let frame = p1[page.p1_index()].pointed_frame().unwrap();
        p1[page.p1_index()].set_unused();
        tlb::flush(VirtAddr::new(page.start_address() as u64));
        allocator.deallocate_frame(frame);
    }
}

pub struct InactivePageTable {
    p4_frame: Frame,
}

impl InactivePageTable {
    pub fn new(
        frame: Frame,
        active_table: &mut ActivePageTable,
        temporary_page: &mut TemporaryPage,
    ) -> InactivePageTable {
        {
            let table = temporary_page.map_table_frame(frame.clone(), active_table);
            table.zero();
            table[511].set(frame.clone(), EntryFlags::PRESENT | EntryFlags::WRITABLE);
        }
        temporary_page.unmap(active_table);

        InactivePageTable { p4_frame: frame }
    }
}

pub fn remap_the_kernel<A>(allocator: &mut A, boot_info: &BootInformation) -> ActivePageTable
where
    A: FrameAllocator,
{
    let mut temporary_page = TemporaryPage::new(Page { number: 0xdeadbeaf }, allocator);

    let mut active_table = unsafe { ActivePageTable::new() };
    let mut new_table = {
        let frame = allocator.allocate_frame().expect("no more frames");
        InactivePageTable::new(frame, &mut active_table, &mut temporary_page)
    };

    active_table.with(&mut new_table, &mut temporary_page, |mapper| {
        let elf_sections_tag = boot_info.elf_sections().expect("Memory map tag required");

        for section in elf_sections_tag {
            if !section.is_allocated() {
                continue;
            }
            assert!(
                section.start_address() as usize % PAGE_SIZE == 0,
                "sections need to be page aligned"
            );

            let flags = EntryFlags::from_elf_section_flags(&section);

            let start_frame = Frame::containing_address(section.start_address() as usize);
            let end_frame = Frame::containing_address(section.end_address() as usize - 1);
            for frame in Frame::range_inclusive(start_frame, end_frame) {
                mapper.identity_map(frame, flags, allocator);
            }
        }

        let vga_buffer_frame = Frame::containing_address(0xb8000);
        mapper.identity_map(vga_buffer_frame, EntryFlags::WRITABLE, allocator);

        let video_start = Frame::containing_address(0xA0000);
        let video_end = Frame::containing_address(0xAF000);

        for frame in Frame::range_inclusive(video_start, video_end) {
            mapper.identity_map(frame, EntryFlags::WRITABLE, allocator);
        }

        let multiboot_start = Frame::containing_address(boot_info.start_address());
        let multiboot_end = Frame::containing_address(boot_info.end_address() - 1);
        for frame in Frame::range_inclusive(multiboot_start, multiboot_end) {
            mapper.identity_map(frame, EntryFlags::PRESENT, allocator);
        }
    });

    let old_table = active_table.switch(new_table);
    let old_p4_page = Page::containing_address(old_table.p4_frame.start_address());
    active_table.unmap(old_p4_page, allocator);

    active_table
}
