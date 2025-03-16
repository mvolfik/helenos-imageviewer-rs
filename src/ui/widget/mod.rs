use crate::helenos;

pub mod image;

pub trait Widget {
    fn get_ctl(&self) -> *mut helenos::ui_control_t;
}
