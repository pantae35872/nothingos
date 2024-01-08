use core::cell::RefCell;

use alloc::rc::Rc;
use alloc::vec::Vec;

use crate::renderer::{FrameRenderer, Renderer};
use crate::vga::VGA;

pub struct RootWidget {
    renderer: Rc<RefCell<FrameRenderer>>,
    childs: Rc<RefCell<Vec<Rc<RefCell<dyn Widget>>>>>,
}

impl RootWidget {
    pub fn new() -> Self {
        Self {
            renderer: Rc::new(RefCell::new(FrameRenderer::new(50, 50))),
            childs: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn render_to_buffer(&mut self) {
        Self::render_all_childs(self.get_childs());
    }

    pub fn render_all_childs(childs: Rc<RefCell<Vec<Rc<RefCell<dyn Widget>>>>>) {
        let childs = childs.borrow_mut();
        if childs.is_empty() {
            return;
        }
        for child in childs.iter() {
            let mut child = child.borrow_mut();
            child.render();
            for y in 0..child.get_renderer().borrow().get_height() {
                for x in 0..child.get_renderer().borrow().get_width() {
                    VGA.put_pixel(x, y, child.get_renderer().borrow().get_at_pos(x, y));
                }
            }

            Self::render_all_childs(child.get_childs());
        }
    }
}

impl Widget for RootWidget {
    fn render(&mut self) {
        for x in 0..50 {
            for y in 0..50 {
                self.renderer.borrow_mut().set_pixel(x, y, 4);
            }
        }
    }

    fn get_childs(&self) -> Rc<RefCell<Vec<Rc<RefCell<dyn Widget>>>>> {
        self.childs.clone()
    }
    fn add_child(&mut self, child: Rc<RefCell<dyn Widget>>) {
        self.childs.borrow_mut().push(child);
    }
    fn get_renderer(&mut self) -> Rc<RefCell<dyn Renderer>> {
        self.renderer.clone()
    }
}

pub struct WindowWidget {
    renderer: Rc<RefCell<FrameRenderer>>,
    childs: Rc<RefCell<Vec<Rc<RefCell<dyn Widget>>>>>,
}

impl WindowWidget {
    pub fn new() -> Self {
        Self {
            renderer: Rc::new(RefCell::new(FrameRenderer::new(100, 100))),
            childs: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl Widget for WindowWidget {
    fn render(&mut self) {
        for x in 0..100 {
            for y in 0..100 {
                self.renderer.borrow_mut().set_pixel(x, y, 8);
            }
        }
    }

    fn get_childs(&self) -> Rc<RefCell<Vec<Rc<RefCell<dyn Widget>>>>> {
        self.childs.clone()
    }
    fn add_child(&mut self, child: Rc<RefCell<dyn Widget>>) {
        self.childs.borrow_mut().push(child);
    }
    fn get_renderer(&mut self) -> Rc<RefCell<dyn Renderer>> {
        self.renderer.clone()
    }
}

pub trait Widget {
    fn render(&mut self);
    fn get_childs(&self) -> Rc<RefCell<Vec<Rc<RefCell<dyn Widget>>>>>;
    fn add_child(&mut self, child: Rc<RefCell<dyn Widget>>);
    fn get_renderer(&mut self) -> Rc<RefCell<dyn Renderer>>;
}
