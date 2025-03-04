#![feature(never_type)]
#![feature(unwrap_infallible)]
#![feature(box_as_ptr)]

use std::process::exit;

use anyhow::{Context, Result};
use image::{ImageBuffer, ImageReader, Rgb};

#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
#[allow(unused)]
mod helenos;
mod libui;

use helenos::util::pointer_init;
use libui::{new_gfx_rect, WindowController};

#[inline(never)]
fn render_image_to_bitmap(
    img: &ImageBuffer<Rgb<u8>, Vec<u8>>,
    ctx: *mut helenos::gfx_context_t,
) -> Result<*mut helenos::gfx_bitmap_t> {
    let mut params = pointer_init(|p| unsafe { helenos::gfx_bitmap_params_init(p) }).into_ok();
    params.rect = new_gfx_rect(img.width() as i32, img.height() as i32);
    let bitmap = pointer_init(|b| unsafe {
        helenos::gfx_bitmap_create(ctx, &mut params, std::ptr::null_mut(), b)
    })
    .context("Failed to create bitmap")?;
    let alloc = pointer_init(|a| unsafe { helenos::gfx_bitmap_get_alloc(bitmap, a) })
        .context("Failed to get bitmap allocation")?;
    let mut pixelmap = helenos::pixelmap_t {
        width: img.width() as usize,
        height: img.height() as usize,
        data: alloc.pixels as *mut helenos::pixel_t,
    };

    println!("Filling image to pixmap {pixelmap:?} within bitmap {bitmap:?}");

    for (x, y, &image::Rgb([r, g, b])) in img.enumerate_pixels() {
        unsafe {
            let pixel = helenos::rgba_to_pix(r, g, b, 0);
            helenos::pixelmap_put_pixel(&mut pixelmap, x as usize, y as usize, pixel);
        }
    }
    Ok(bitmap)
}

const W: u32 = 400;
const H: u32 = 500;

struct Ctx {}

impl libui::WindowUserController for Ctx {
    fn on_close(&mut self, controller: &mut WindowController<'_>) {
        controller.ui().quit();
    }

    fn on_resize(&mut self, controller: &mut WindowController<'_>) {
        controller.paint();
    }

    fn on_maximize(&mut self, controller: &mut WindowController<'_>) {
        self.on_resize(controller);
    }

    fn on_unmaximize(&mut self, controller: &mut WindowController<'_>) {
        self.on_resize(controller);
    }
}

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        println!("Provide a path of file to view");
        exit(1);
    };

    let ui = libui::Ui::new().expect("Failed to create UI");
    let window = ui
        .create_window(c"Image Viewer", W, H, Ctx {})
        .expect("Failed to create window");

    let mut img = ImageReader::open(&path)
        .expect("Failed to open image")
        .decode()
        .expect("Failed to decode image")
        .into_rgb8();
    let ow = img.width();
    let oh = img.height();
    if ow > W || oh > H {
        let (w, h) = if (W * oh) / ow <= H {
            (W, (W * oh) / ow)
        } else {
            ((H * ow) / oh, H)
        };
        img = image::imageops::resize(&img, w, h, image::imageops::FilterType::Triangle);
    }

    let graphic_context = window.get_gc();
    let bitmap =
        render_image_to_bitmap(&img, graphic_context).expect("Failed to render image to bitmap");

    let mut rect = new_gfx_rect(img.width() as i32, img.height() as i32);
    println!("Image rect for image_create: {:?}", rect);
    let image = pointer_init(|i| unsafe {
        helenos::ui_image_create(window.get_res(), bitmap, &mut rect, i)
    })
    .expect("Failed to create image");

    let mut rect = window.get_app_rect();
    println!("App rect: {:?}", rect);
    // center image within the window
    let horizontal_diff = (rect.p1.x - rect.p0.x) - img.width() as i32;
    let vertical_diff = (rect.p1.y - rect.p0.y) - img.height() as i32;
    let left_margin = horizontal_diff / 2;
    let top_margin = vertical_diff / 2;
    rect.p0.x += left_margin;
    rect.p0.y += top_margin;
    rect.p1.x -= horizontal_diff - left_margin;
    rect.p1.y -= vertical_diff - top_margin;
    println!("Image rect: {:?}", rect);

    unsafe {
        helenos::ui_image_set_rect(image, &mut rect);
        window.add_widget(helenos::ui_image_ctl(image));
        window.controller().paint();
        assert!(!window.is_borrowed());
        assert!(!window.is_mut_borrowed());
        println!("starting app run");
        ui.run();
    }
    println!("Program done");
}
