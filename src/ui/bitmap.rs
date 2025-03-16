use crate::helenos::{self, util::pointer_init};

use super::new_gfx_rect;

pub struct Bitmap {
    w: u32,
    h: u32,
    pub(super) bmp: *mut helenos::gfx_bitmap_t,
}

impl Drop for Bitmap {
    fn drop(&mut self) {
        println!("Dropping Bitmap");
        unsafe {
            helenos::gfx_bitmap_destroy(self.bmp);
        }
    }
}

impl Bitmap {
    pub fn new(ctx: *mut helenos::gfx_context_t, w: u32, h: u32) -> Result<Self, std::io::Error> {
        let mut params = pointer_init(|p| unsafe { helenos::gfx_bitmap_params_init(p) }).into_ok();
        params.rect = new_gfx_rect(w, h);
        let bmp = pointer_init(|b| unsafe {
            helenos::gfx_bitmap_create(ctx, &mut params, std::ptr::null_mut(), b)
        })?;
        Ok(Self { w, h, bmp })
    }

    pub fn w(&self) -> u32 {
        self.w
    }

    pub fn h(&self) -> u32 {
        self.h
    }

    pub fn pixelmap(&mut self) -> Result<PixelMap, std::io::Error> {
        let pixelmap = helenos::pixelmap_t {
            width: self.w as usize,
            height: self.h as usize,
            data: pointer_init(|a| unsafe { helenos::gfx_bitmap_get_alloc(self.bmp, a) })?.pixels
                as *mut helenos::pixel_t,
        };
        Ok(PixelMap(self, pixelmap))
    }
}

pub struct PixelMap<'a>(&'a mut Bitmap, helenos::pixelmap_t);

impl<'a> PixelMap<'a> {
    pub fn set_pixel_rgba(&mut self, (x, y): (usize, usize), (r, g, b, a): (u8, u8, u8, u8)) {
        unsafe {
            let pixel = helenos::rgba_to_pix(r, g, b, a);
            helenos::pixelmap_put_pixel(&mut self.1, x, y, pixel);
        }
    }
}
