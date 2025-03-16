use crate::helenos::{self, util::pointer_init};

use super::super::{bitmap::Bitmap, new_gfx_rect, UiResource};
use super::Widget;

pub struct Image(*mut helenos::ui_image_t);

impl Widget for Image {
    fn get_ctl(&self) -> *mut crate::helenos::ui_control_t {
        unsafe { crate::helenos::ui_image_ctl(self.0) }
    }
}

impl Image {
    pub fn new(
        res: UiResource,
        bitmap: Bitmap,
        rect: &mut helenos::gfx_rect_t,
    ) -> Result<Self, std::io::Error> {
        let image = pointer_init(|i| unsafe {
            crate::helenos::ui_image_create(
                res.0,
                bitmap.bmp,
                &mut new_gfx_rect(bitmap.w(), bitmap.h()),
                i,
            )
        })?;
        std::mem::forget(bitmap); // it is now owned and freed by the image
        unsafe { helenos::ui_image_set_rect(image, rect) };
        Ok(Self(image))
    }
}
