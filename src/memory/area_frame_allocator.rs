use crate::memory::{Frame, FrameAllocator};
use crate::println;
use multiboot2::MemoryArea;

pub struct AreaFrameAllocator<'a> {
    next_free_frame: Frame,
    current_area: Option<MemoryArea>,
    areas: &'a [MemoryArea],
    kernel_start: Frame,
    kernel_end: Frame,
    multiboot_start: Frame,
    multiboot_end: Frame,
}

impl<'a> AreaFrameAllocator<'a> {
    pub fn new(
        kernel_start: usize,
        kernel_end: usize,
        multiboot_start: usize,
        multiboot_end: usize,
        memory_areas: &[MemoryArea],
    ) -> AreaFrameAllocator {
        let mut allocator = AreaFrameAllocator {
            next_free_frame: Frame::containing_address(0),
            current_area: None,
            areas: memory_areas,
            kernel_start: Frame::containing_address(kernel_start),
            kernel_end: Frame::containing_address(kernel_end),
            multiboot_start: Frame::containing_address(multiboot_start),
            multiboot_end: Frame::containing_address(multiboot_end),
        };

        allocator.choose_next_area();
        allocator
    }

    fn choose_next_area(&mut self) {
        self.current_area = self
            .areas
            .iter()
            .filter(|area| {
                let address = area.start_address() + area.size() - 1;
                Frame::containing_address(address as usize) >= self.next_free_frame
            })
            .min_by_key(|area| area.start_address())
            .copied();

        if let Some(area) = self.current_area {
            let start_frame = Frame::containing_address(area.start_address() as usize);
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
                let address = area.start_address() + area.size() - 1;
                Frame::containing_address(address as usize)
            };

            if frame > current_area_last_frame {
                self.choose_next_area();
            } else if frame >= self.kernel_start && frame <= self.kernel_end {
                self.next_free_frame = Frame {
                    number: self.kernel_end.number + 1,
                };
            } else if frame >= self.multiboot_start && frame <= self.multiboot_end {
                self.next_free_frame = Frame {
                    number: self.multiboot_end.number + 1,
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
