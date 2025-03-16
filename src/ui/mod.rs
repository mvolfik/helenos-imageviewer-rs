use crate::helenos;

pub mod bitmap;
pub mod widget;
pub mod window_controller;

mod ui;
pub use ui::*;

pub fn new_gfx_rect(w: u32, h: u32) -> helenos::gfx_rect_t {
    helenos::gfx_rect_t {
        p0: helenos::gfx_coord2_t { x: 0, y: 0 },
        p1: helenos::gfx_coord2_t {
            x: w as i32,
            y: h as i32,
        },
    }
}
