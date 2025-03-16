use std::collections::HashMap;

use crate::helenos;

use super::{widget::Widget, Ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetID(u32);

pub struct WindowController<'a> {
    pub(super) ui: &'a Ui,
    pub(super) window: *mut helenos::ui_window_t,
    pub(super) next_widget_id: u32,
    pub(super) widgets: HashMap<WidgetID, Box<dyn Widget>>,
}

impl WindowController<'_> {
    pub fn ui(&mut self) -> &Ui {
        self.ui
    }

    pub fn paint(&mut self) {
        unsafe {
            helenos::ui_window_paint(self.window);
        }
    }
}

pub struct WindowInner<'a, T: WindowUserController> {
    pub(super) controller: WindowController<'a>,
    pub(super) user_controller: T,
}

impl<T: WindowUserController> Drop for WindowInner<'_, T> {
    fn drop(&mut self) {
        println!("Dropping WindowInner");
        unsafe {
            helenos::ui_window_destroy(self.controller.window);
        }
    }
}

#[allow(unused)]
pub trait WindowUserController: Sized {
    fn on_close(&mut self, controller: &mut WindowController<'_>) {}
    fn on_resize(&mut self, controller: &mut WindowController<'_>) {}
    fn on_maximize(&mut self, controller: &mut WindowController<'_>) {}
    fn on_unmaximize(&mut self, controller: &mut WindowController<'_>) {}
    fn on_kbd(&mut self, controller: &mut WindowController<'_>, event: *mut helenos::kbd_event_t) {}
}
