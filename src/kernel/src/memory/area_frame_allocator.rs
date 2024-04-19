use crate::memory::{Frame, FrameAllocator};
use uefi::table::boot::{MemoryDescriptor, MemoryMap, MemoryType};

pub struct AreaFrameAllocator<'a> {
    next_free_frame: Frame,
    current_area: Option<MemoryDescriptor>,
    areas: &'a MemoryMap<'static>,
    kernel_start: Frame,
    kernel_end: Frame,
    apic_start: Frame,
    apic_end: Frame,
}

impl<'a> AreaFrameAllocator<'a> {
    pub fn new(
        kernel_start: usize,
        kernel_end: usize,
        apic_start: usize,
        apic_end: usize,
        memory_areas: &'a MemoryMap<'static>,
    ) -> AreaFrameAllocator<'a> {
        let mut allocator = AreaFrameAllocator {
            next_free_frame: Frame::containing_address(0),
            current_area: None,
            areas: memory_areas,
            kernel_start: Frame::containing_address(kernel_start),
            kernel_end: Frame::containing_address(kernel_end),
            apic_start: Frame::containing_address(apic_start),
            apic_end: Frame::containing_address(apic_end),
        };

        allocator.choose_next_area();
        allocator
    }

    fn choose_next_area(&mut self) {
        self.current_area = self
            .areas
            .entries()
            .filter(|area| {
                let address = area.phys_start + (area.page_count * 4096) - 1;
                Frame::containing_address(address as usize) >= self.next_free_frame
                    && area.ty == MemoryType::CONVENTIONAL
            })
            .min_by_key(|area| area.phys_start)
            .copied();

        if let Some(area) = self.current_area {
            let start_frame = Frame::containing_address(area.phys_start as usize);
            if self.next_free_frame < start_frame {
                self.next_free_frame = start_frame;
            }
        }
    }
}

impl<'a> FrameAllocator for AreaFrameAllocator<'a> {
    fn allocate_frame(&mut self) -> Option<Frame> {
        if let Some(area) = self.current_area {
            let frame = Frame {
                number: self.next_free_frame.number,
            };

            let current_area_last_frame = {
                let address = area.phys_start + area.page_count * 4096 - 1;
                Frame::containing_address(address as usize)
            };

            if frame > current_area_last_frame {
                self.choose_next_area();
            } else if frame >= self.kernel_start && frame <= self.kernel_end {
                self.next_free_frame = Frame {
                    number: self.kernel_end.number + 1,
                };
            } else if frame >= self.apic_start && frame <= self.apic_end {
                self.next_free_frame = Frame {
                    number: self.apic_end.number + 1,
                };
            } else {
                self.next_free_frame.number += 1;
                return Some(frame);
            }
            self.allocate_frame()
        } else {
            None
        }
    }

    fn deallocate_frame(&mut self, _frame: Frame) {}
}
