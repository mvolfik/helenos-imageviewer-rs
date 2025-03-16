use std::{
    cell::{RefCell, RefMut},
    collections::HashMap,
    ffi::CStr,
};

use crate::helenos::{self, util::pointer_init};

use super::{
    new_gfx_rect,
    widget::Widget,
    window_controller::{WindowController, WindowInner, WindowUserController},
};

pub struct Ui(*mut helenos::ui_t);

impl Drop for Ui {
    fn drop(&mut self) {
        println!("Dropping Ui");
        unsafe {
            helenos::ui_destroy(self.0);
        }
    }
}

impl Ui {
    pub fn new() -> Result<Self, std::io::Error> {
        let ui = pointer_init(|ui| unsafe {
            helenos::ui_create(helenos::UI_DISPLAY_DEFAULT.as_ptr() as *const i8, ui)
        })?;
        Ok(Self(ui))
    }

    pub fn create_window<'a, T: WindowUserController>(
        &'a self,
        caption: &CStr,
        width: u32,
        height: u32,
        user_controller: T,
    ) -> Result<Window<'a, T>, std::io::Error> {
        let mut window_params =
            pointer_init(|wp| unsafe { helenos::ui_wnd_params_init(wp) }).into_ok();
        window_params.rect =
            get_window_rectangle_for_size(self.0, width, height, window_params.style);
        window_params.caption = caption.as_ptr();
        window_params.style |= helenos::ui_wdecor_style_t::ui_wds_resizable
            | helenos::ui_wdecor_style_t::ui_wds_maximize_btn;

        let window: Window<'a, T> = Window(Box::new(RefCell::new(WindowInner {
            controller: WindowController {
                ui: self,
                window: pointer_init(|w| unsafe {
                    helenos::ui_window_create(self.0, &mut window_params, w)
                })?,
                next_widget_id: 10,
                widgets: HashMap::new(),
            },
            user_controller,
        })));
        let ptr: CallbackType<'a, T> = Box::as_ptr(&window.0);

        unsafe {
            helenos::ui_window_set_cb(
                window.0.borrow().controller.window,
                &Window::<'a, T>::CALLBACKS as *const helenos::ui_window_cb_t
                    as *mut helenos::ui_window_cb_t,
                ptr as *mut std::ffi::c_void,
            );
        }
        Ok(window)
    }

    pub fn run(&self) {
        unsafe {
            helenos::ui_run(self.0);
        }
        println!("Ui run finished");
    }

    pub fn quit(&self) {
        unsafe {
            helenos::ui_quit(self.0);
        }
    }
}

pub struct Window<'a, T: WindowUserController>(Box<RefCell<WindowInner<'a, T>>>);

trait CallbacksProvider {
    const CALLBACKS: helenos::ui_window_cb_t;
}

impl<'a, T: WindowUserController> CallbacksProvider for Window<'a, T> {
    const CALLBACKS: helenos::ui_window_cb_t = helenos::ui_window_cb_t {
        close: Some(Self::on_close),
        // kbd: Some(Self::on_kbd),
        resize: Some(Self::on_resize),
        maximize: Some(Self::on_maximize),
        unmaximize: Some(Self::on_unmaximize),

        ..unsafe { std::mem::zeroed() }
    };
}

type CallbackType<'a, T> = *const RefCell<WindowInner<'a, T>>;

macro_rules! impl_callbacks {
    { $(fn $name:ident($w:ident $(,)? $($arg:ident: $arg_ty:ty),*) $body:block)* } => {
        $(
            unsafe extern "C" fn $name(
                $w: *mut helenos::ui_window_t,
                arg: *mut std::ffi::c_void,
                $($arg: $arg_ty),*
            ) {
                let window = &*(arg as CallbackType<'a, T>);
                println!("Callback {} called", stringify!($name));
                let (mut controller, mut user_controller) = RefMut::map_split(
                    window.borrow_mut(),
                    |b| (&mut b.controller, &mut b.user_controller)
                );
                debug_assert_eq!(controller.window, $w);
                {
                    $body
                }
                user_controller.$name(&mut *controller, $($arg),*);
            }
        )*
    };
}

pub struct UiResource(pub(super) *mut helenos::ui_resource_t);

impl<'a, T: WindowUserController> Window<'a, T> {
    impl_callbacks! {
        fn on_close(w) {}
        fn on_kbd(w, event: *mut helenos::kbd_event_t) {
            let claim = helenos::ui_window_def_kbd(w, event);
            if claim != helenos::ui_evclaim_t_ui_unclaimed {
                return;
            }
        }

        fn on_resize(w) {
            helenos::ui_window_def_resize(w);
        }
        fn on_maximize(w) {
            helenos::ui_window_def_maximize(w);
        }
        fn on_unmaximize(w) {
            helenos::ui_window_def_unmaximize(w);
        }
    }

    pub fn get_resource(&self) -> UiResource {
        UiResource(unsafe { helenos::ui_window_get_res(self.0.borrow().controller.window) })
    }
    pub fn get_gc(&self) -> *mut helenos::gfx_context_t {
        unsafe { helenos::ui_window_get_gc(self.0.borrow().controller.window) }
    }
    pub fn add_widget(&self, widget: impl Widget) {
        unsafe {
            helenos::ui_window_add(self.0.borrow().controller.window, widget.get_ctl());
        }
    }
    pub fn get_app_rect(&self) -> helenos::gfx_rect_t {
        unsafe {
            pointer_init(|rect| {
                helenos::ui_window_get_app_rect(self.0.borrow().controller.window, rect);
            })
            .into_ok()
        }
    }
    pub fn controller<'b>(&'b self) -> RefMut<'b, WindowController<'a>> {
        RefMut::map(self.0.borrow_mut(), |b| &mut b.controller)
    }
    pub fn user_controller<'b>(&'b self) -> RefMut<'b, T> {
        RefMut::map(self.0.borrow_mut(), |b| &mut b.user_controller)
    }
    pub fn controllers<'b>(&'b self) -> (RefMut<'b, WindowController<'a>>, RefMut<'b, T>) {
        RefMut::map_split(self.0.borrow_mut(), |b| {
            (&mut b.controller, &mut b.user_controller)
        })
    }
    pub fn is_borrowed(&self) -> bool {
        self.0.try_borrow().is_err()
    }
    pub fn is_mut_borrowed(&self) -> bool {
        self.0.try_borrow_mut().is_err()
    }
}

fn get_window_rectangle_for_size(
    ui: *mut helenos::ui_t,
    width: u32,
    height: u32,
    style: helenos::ui_wdecor_style_t,
) -> helenos::gfx_rect_t {
    let mut naive_rect = new_gfx_rect(width, height);
    let mut window_rect = new_gfx_rect(0, 0);
    unsafe {
        helenos::ui_wdecor_rect_from_app(ui, style, &mut naive_rect, &mut window_rect);
        // now window rectangle starts in (-x,-y) so that application can draw to the 0,0...w,h area
        // -> use the (-x,-y) coordinate as an offset for an inverse move of the window

        let mut off = window_rect.p0; // can't inline this or we get aliasing pointers
        let mut final_rect = new_gfx_rect(0, 0);
        helenos::gfx_rect_rtranslate(&mut off, &mut window_rect, &mut final_rect);
        final_rect
    }
}
