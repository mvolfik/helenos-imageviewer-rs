#![feature(never_type)]
#![feature(unwrap_infallible)]
#![feature(box_as_ptr)]

use std::{mem::MaybeUninit, process::exit, cell::RefCell};

use anyhow::{Context, Result};
use image::{ImageBuffer, ImageReader, Rgb};

#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
#[allow(unused)]
mod helenos;

fn new_gfx_rect() -> helenos::gfx_rect_t {
    helenos::gfx_rect_t {
        p0: helenos::gfx_coord2_t { x: 0, y: 0 },
        p1: helenos::gfx_coord2_t { x: 0, y: 0 },
    }
}

fn get_window_rectangle_for_size(
    ui: *mut helenos::ui_t,
    width: u32,
    height: u32,
    style: helenos::ui_wdecor_style_t,
) -> helenos::gfx_rect_t {
    let mut naive_rect = helenos::gfx_rect_t {
        p0: helenos::gfx_coord2_t { x: 0, y: 0 },
        p1: helenos::gfx_coord2_t {
            x: width as i32,
            y: height as i32,
        },
    };
    let mut window_rect = new_gfx_rect();
    unsafe {
        helenos::ui_wdecor_rect_from_app(ui, style, &mut naive_rect, &mut window_rect);
        // now window rectangle starts in (-x,-y) so that application can draw to the 0,0...w,h area
        // -> use the (-x,-y) coordinate as an offset for an inverse move of the window

        let mut off = window_rect.p0; // can't inline this or we get aliasing pointers
        let mut final_rect = new_gfx_rect();
        helenos::gfx_rect_rtranslate(&mut off, &mut window_rect, &mut final_rect);
        final_rect
    }
}

trait IntoError {
    type Error: std::error::Error;
    fn into_error(self) -> Result<(), Self::Error>;
}

impl IntoError for helenos::errno_t {
    type Error = std::io::Error;

    fn into_error(self) -> Result<(), std::io::Error> {
        if self == 0 {
            Ok(())
        } else {
            Err(std::io::Error::from_raw_os_error(self))
        }
    }
}

impl IntoError for () {
    type Error = !;

    fn into_error(self) -> Result<(), !> {
        Ok(())
    }
}

fn pointer_init<T, E: IntoError, F: FnOnce(*mut T) -> E>(fun: F) -> Result<T, E::Error> {
    let mut uninit = MaybeUninit::uninit();
    fun(uninit.as_mut_ptr()).into_error()?;
    Ok(unsafe { uninit.assume_init() })
}

fn rectangle_for_image(img: &ImageBuffer<Rgb<u8>, Vec<u8>>) -> helenos::gfx_rect_t {
    helenos::gfx_rect_t {
        p0: helenos::gfx_coord2_t { x: 0, y: 0 },
        p1: helenos::gfx_coord2_t {
            x: img.width() as i32,
            y: img.height() as i32,
        },
    }
}

#[inline(never)]
fn render_image_to_bitmap(
    img: &ImageBuffer<Rgb<u8>, Vec<u8>>,
    ctx: *mut helenos::gfx_context_t,
) -> Result<*mut helenos::gfx_bitmap_t> {
    let mut params = pointer_init(|p| unsafe { helenos::gfx_bitmap_params_init(p) }).into_ok();
    params.rect = rectangle_for_image(img);
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

// fn render_image(path: impl AsRef<Path>, window: helenos::ui_window_t) -> Result<()> {
//     let img = ImageReader::open(&path)
//         .context("Failed to open image")?
//         .decode()
//         .context("Failed to decode image")?
//         .into_rgb8();
//     Ok(())
// }

const W: u32 = 400;
const H: u32 = 500;

// pub struct WindowController<T: WindowUserController> {
//     ui: *mut helenos::ui_t,
//     window: *mut helenos::ui_window_t,
//     user_controller
// }

// pub trait WindowUserController {
//     fn on_close
// }

struct Ctx {
    ui: *mut helenos::ui_t,
    window: *mut helenos::ui_window_t,
}

type WindowHandlerArg = *const RefCell<Ctx>;

fn handle_close(ctx: &RefCell<Ctx>) {
    println!("Window closed");
    unsafe { helenos::ui_quit(ctx.borrow().ui) };
}

unsafe extern "C" fn on_close(_w: *mut helenos::ui_window_t, arg: *mut std::ffi::c_void) {
    handle_close(&*(arg as WindowHandlerArg));
}

fn handle_resize(ctx: &RefCell<Ctx>) {
    println!("Window resized");
    let ctx = ctx.borrow();
    unsafe { helenos::ui_window_paint(ctx.window) };
}

unsafe extern "C" fn on_resize(
    w: *mut helenos::ui_window_t,
    arg: *mut std::ffi::c_void,
) {
    helenos::ui_window_def_resize(w);
    handle_resize(&*(arg as WindowHandlerArg));
}

unsafe extern "C" fn on_maximize(
    w: *mut helenos::ui_window_t,
    arg: *mut std::ffi::c_void,
) {
    helenos::ui_window_def_maximize(w);
    handle_resize(&*(arg as WindowHandlerArg));
}

unsafe extern "C" fn on_unmaximize(
    w: *mut helenos::ui_window_t,
    arg: *mut std::ffi::c_void,
) {
    helenos::ui_window_def_unmaximize(w);
    handle_resize(&*(arg as WindowHandlerArg));
}

unsafe extern "C" fn on_kbd(
    _w: *mut helenos::ui_window_t,
    _arg: *mut std::ffi::c_void,
    event: *mut helenos::kbd_event_t,
) {
    println!("Key event: {:?}", unsafe { &*event });
}

const WINDOW_CALLBACKS: helenos::ui_window_cb_t = helenos::ui_window_cb_t {
    close: Some(on_close),
    kbd: Some(on_kbd),

    resize: Some(on_resize),
    maximize: Some(on_maximize),
    unmaximize: Some(on_unmaximize),

    ..unsafe { std::mem::zeroed() }
};

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        println!("Provide a path of file to view");
        exit(1);
    };

    let ui = pointer_init(|ui| unsafe {
        helenos::ui_create(helenos::UI_DISPLAY_DEFAULT.as_ptr() as *const i8, ui)
    })
    .expect("Failed to create UI");

    let mut window_params = pointer_init(|wp| unsafe { helenos::ui_wnd_params_init(wp) }).into_ok();
    window_params.rect = get_window_rectangle_for_size(ui, W, H, window_params.style);
    window_params.caption = c"Rust image viewer".as_ptr();
    window_params.style |= helenos::ui_wdecor_style_t::ui_wds_resizable | helenos::ui_wdecor_style_t::ui_wds_maximize_btn;
    println!("window rect: {:?}", window_params.rect);

    let window = pointer_init(|w| unsafe { helenos::ui_window_create(ui, &mut window_params, w) })
        .expect("Failed to create window");
    let graphic_context = unsafe { helenos::ui_window_get_gc(window) };

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
    let bitmap =
        render_image_to_bitmap(&img, graphic_context).expect("Failed to render image to bitmap");

    let mut image_rect = rectangle_for_image(&img);
    println!("Image rect for image_create: {:?}", image_rect);
    let image = pointer_init(|i| unsafe {
        helenos::ui_image_create(
            helenos::ui_window_get_res(window),
            bitmap,
            &mut image_rect,
            i,
        )
    })
    .expect("Failed to create image");

    let mut image_rect = pointer_init(|ar| unsafe { helenos::ui_window_get_app_rect(window, ar) })
        .expect("Failed to get app rectangle");
    println!("App rect: {:?}", image_rect);
    // center image within the window
    let horizontal_diff = (image_rect.p1.x - image_rect.p0.x) - img.width() as i32;
    let vertical_diff = (image_rect.p1.y - image_rect.p0.y) - img.height() as i32;
    let left_margin = horizontal_diff / 2;
    let top_margin = vertical_diff / 2;
    image_rect.p0.x += left_margin;
    image_rect.p0.y += top_margin;
    image_rect.p1.x -= horizontal_diff - left_margin;
    image_rect.p1.y -= vertical_diff - top_margin;
    println!("Image rect: {:?}", image_rect);

    let ctx = RefCell::new(Ctx { ui, window });

    unsafe {
        helenos::ui_window_set_cb(window, &WINDOW_CALLBACKS as *const _ as *mut _, &ctx as WindowHandlerArg as *mut std::ffi::c_void);
        helenos::ui_image_set_rect(image, &mut image_rect);
        helenos::ui_window_add(window, helenos::ui_image_ctl(image));
        helenos::ui_window_paint(window);
        helenos::ui_run(ui);
        helenos::ui_window_destroy(window);
        helenos::ui_destroy(ui);
    }
    println!("Program done");
}
