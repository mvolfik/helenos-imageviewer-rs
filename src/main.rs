use core::mem::MaybeUninit;

use image::ImageReader;

#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
#[allow(unused)]
mod helenos;

macro_rules! cvt {
    ($e:expr) => {
        let res = $e;
        if res != 0 {
            panic!("Error calling {}: {}", stringify!($e), res);
        }
    };
}

fn main() {
    let argv = std::env::args().collect::<Vec<_>>();
    let path = argv.get(1).cloned().unwrap_or("a.png".to_owned());

    let ui = unsafe {
        let mut ui = MaybeUninit::uninit();
        cvt!(helenos::ui_create(
            helenos::UI_DISPLAY_DEFAULT.as_ptr() as *const i8,
            ui.as_mut_ptr()
        ));
        ui.assume_init()
    };
    let mut window_params = unsafe {
        let mut window_params = MaybeUninit::uninit();
        helenos::ui_wnd_params_init(window_params.as_mut_ptr());
        window_params.assume_init()
    };

    let mut app_rect = helenos::gfx_rect_t {
        p0: helenos::gfx_coord2_t { x: 0, y: 0 },
        p1: helenos::gfx_coord2_t { x: 100, y: 200 },
    };
    let mut window_rect = helenos::gfx_rect_t {
        p0: helenos::gfx_coord2_t { x: 0, y: 0 },
        p1: helenos::gfx_coord2_t { x: 0, y: 0 },
    };

    unsafe {
        helenos::ui_wdecor_rect_from_app(ui, window_params.style, &mut app_rect, &mut window_rect);
        let mut off = window_rect.p0; // can't inline this or we get aliasing pointers
        helenos::gfx_rect_rtranslate(&mut off, &mut window_rect, &mut window_params.rect);
    };

    window_params.caption = c"Rust image viewer".as_ptr();

    let window = unsafe {
        let mut window = MaybeUninit::uninit();
        cvt!(helenos::ui_window_create(
            ui,
            &mut window_params,
            window.as_mut_ptr()
        ));
        window.assume_init()
    };
    let graphic_context = unsafe { helenos::ui_window_get_gc(window) };

    let img = ImageReader::open(&path)
        .expect("Failed to open imageea")
        .decode()
        .expect("Failed to decode image")
        .into_rgb8();

    let mut bitmap_params = unsafe {
        let mut bitmap_params = MaybeUninit::uninit();
        helenos::gfx_bitmap_params_init(bitmap_params.as_mut_ptr());
        bitmap_params.assume_init()
    };
    bitmap_params.rect.p0.x = 0;
    bitmap_params.rect.p0.y = 0;
    bitmap_params.rect.p1.x = img.width() as i32;
    bitmap_params.rect.p1.y = img.height() as i32;

    let bitmap = unsafe {
        let mut bitmap = MaybeUninit::uninit();
        helenos::gfx_bitmap_create(
            graphic_context,
            &mut bitmap_params,
            std::ptr::null_mut(),
            bitmap.as_mut_ptr(),
        );
        bitmap.assume_init()
    };
    let mut bitmap_alloc = unsafe {
        let mut bitmap_alloc = MaybeUninit::uninit();
        helenos::gfx_bitmap_get_alloc(bitmap, bitmap_alloc.as_mut_ptr());
        bitmap_alloc.assume_init()
    };

    let mut pixelmap = helenos::pixelmap_t {
        width: img.width() as usize,
        height: img.height() as usize,
        data: bitmap_alloc.pixels as *mut helenos::pixel_t,
    };

    for (x, y, &image::Rgb([r, g, b])) in img.enumerate_pixels() {
        unsafe {
            let pixel = helenos::rgba_to_pix(r, g, b, 0);
            helenos::pixelmap_put_pixel(&mut pixelmap, x as usize, y as usize, pixel);
        }
    }

    let bitmap = unsafe {
        let mut bitmap = MaybeUninit::uninit();
        cvt!(helenos::gfx_bitmap_create(
            graphic_context,
            &mut bitmap_params,
            &mut bitmap_alloc,
            bitmap.as_mut_ptr(),
        ));
        bitmap.assume_init()
    };

    let resource = unsafe { helenos::ui_window_get_res(window) };
    let image = unsafe {
        let mut image = MaybeUninit::uninit();
        cvt!(helenos::ui_image_create(resource, bitmap, &mut app_rect, image.as_mut_ptr()));
        image.assume_init()
    };
    let mut app_rect2 = unsafe {
        let mut app_rect2 = MaybeUninit::uninit();
        helenos::ui_window_get_app_rect(window, app_rect2.as_mut_ptr());
        app_rect2.assume_init()
    };
    unsafe {
        helenos::ui_image_set_rect(image, &mut app_rect2);
        helenos::ui_window_add(window, helenos::ui_image_ctl(image));
        helenos::ui_window_paint(window);
        helenos::ui_run(ui);
        helenos::ui_window_destroy(window);
        helenos::ui_destroy(ui);
    }
}
