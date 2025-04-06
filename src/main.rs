#![feature(never_type)]
#![feature(unwrap_infallible)]
#![feature(box_as_ptr)]

use std::process::exit;

use crate::ui::{
    bitmap::Bitmap,
    new_gfx_rect,
    widget::image::Image,
    window_controller::{WindowController, WindowUserController},
    Ui,
};

#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
#[allow(unused)]
mod helenos;
mod ui;

const W: u32 = 400;
const H: u32 = 500;

struct Ctx {
    img: image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
}

impl WindowUserController for Ctx {
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

    let ui = Ui::new().expect("Failed to create UI");

    let mut img = image::ImageReader::open(&path)
        .expect("Failed to open image")
        .decode()
        .expect("Failed to decode image")
        .into_rgb8();

    let window = ui
        .create_window(c"Image Viewer", W, H, Ctx { img: img.clone() })
        .expect("Failed to create window");

    {
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
    }

    let mut bitmap =
        Bitmap::new(window.get_gc(), img.width(), img.height()).expect("Failed to create bitmap");

    let mut pixmap = bitmap.pixelmap().expect("Failed to get pixelmap");
    for (x, y, &image::Rgb([r, g, b])) in img.enumerate_pixels() {
        pixmap.set_pixel_rgba((x as usize, y as usize), (r, g, b, 0));
    }

    let mut rect = window.get_app_rect();
    // center image within the window
    let horizontal_diff = (rect.p1.x - rect.p0.x) - img.width() as i32;
    let vertical_diff = (rect.p1.y - rect.p0.y) - img.height() as i32;
    let left_margin = horizontal_diff / 2;
    let top_margin = vertical_diff / 2;
    rect.p0.x += left_margin;
    rect.p0.y += top_margin;
    rect.p1.x -= horizontal_diff - left_margin;
    rect.p1.y -= vertical_diff - top_margin;

    let image =
        Image::new(window.get_resource(), bitmap, &mut rect).expect("Failed to create image");

    window.add_widget(image);
    window.controller().paint();
    ui.run();
    println!("Program done");
}
