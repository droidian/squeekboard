/*! Drawing the UI */

use cairo;
use std::cell::RefCell;

use ::action::Action;
use ::keyboard;
use ::layout::{ Button, Label, Layout };
use ::layout::c::{ Bounds, EekGtkKeyboard, Point };
use ::submission::Submission;

use glib::translate::FromGlibPtrNone;
use gtk::WidgetExt;

use std::ffi::CStr;
use std::ptr;

mod c {
    use super::*;

    use cairo_sys;
    use std::os::raw::{ c_char, c_void };
    
    // This is constructed only in C, no need for warnings
    #[allow(dead_code)]
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    pub struct EekRenderer(*const c_void);

    // This is constructed only in C, no need for warnings
    /// Just don't clone this for no reason.
    #[allow(dead_code)]
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    pub struct GtkStyleContext(*const c_void);


    #[no_mangle]
    extern "C" {
        #[allow(improper_ctypes)]
        pub fn eek_renderer_get_scale_factor(
            renderer: EekRenderer,
        ) -> u32;

        #[allow(improper_ctypes)]
        pub fn eek_render_button_in_context(
            scale_factor: u32,
            cr: *mut cairo_sys::cairo_t,
            ctx: GtkStyleContext,
            bounds: Bounds,
            icon_name: *const c_char,
            label: *const c_char,
        );

        #[allow(improper_ctypes)]
        pub fn eek_get_style_context_for_button(
            renderer: EekRenderer,
            name: *const c_char,
            outline_name: *const c_char,
            pressed: u64,
            locked: u64,
        ) -> GtkStyleContext;

        #[allow(improper_ctypes)]
        pub fn eek_put_style_context_for_button(
            ctx: GtkStyleContext,
            outline_name: *const c_char,
            locked: u64,
        );
    }

    /// Draws all buttons that are not in the base state
    #[no_mangle]
    pub extern "C"
    fn squeek_layout_draw_all_changed(
        layout: *mut Layout,
        renderer: EekRenderer,
        cr: *mut cairo_sys::cairo_t,
        submission: *const Submission,
    ) {
        let layout = unsafe { &mut *layout };
        let submission = unsafe { &*submission };
        let cr = unsafe { cairo::Context::from_raw_none(cr) };
        let active_modifiers = submission.get_active_modifiers();

        layout.foreach_visible_button(|offset, button| {
            let state = RefCell::borrow(&button.state).clone();
            let active_mod = match &state.action {
                Action::ApplyModifier(m) => active_modifiers.contains(m),
                _ => false,
            };
            let locked = state.action.is_active(&layout.current_view)
                | active_mod;
            if state.pressed == keyboard::PressType::Pressed || locked {
                render_button_at_position(
                    renderer, &cr,
                    offset,
                    button.as_ref(),
                    state.pressed, locked,
                );
            }
        })
    }
    
    #[no_mangle]
    pub extern "C"
    fn squeek_draw_layout_base_view(
        layout: *mut Layout,
        renderer: EekRenderer,
        cr: *mut cairo_sys::cairo_t,
    ) {
        let layout = unsafe { &mut *layout };
        let cr = unsafe { cairo::Context::from_raw_none(cr) };
        
        layout.foreach_visible_button(|offset, button| {
            render_button_at_position(
                renderer, &cr,
                offset,
                button.as_ref(),
                keyboard::PressType::Released, false,
            );
        })
    }
}

/// Renders a button at a position (button's own bounds ignored)
pub fn render_button_at_position(
    renderer: c::EekRenderer,
    cr: &cairo::Context,
    position: Point,
    button: &Button,
    pressed: keyboard::PressType,
    locked: bool,
) {
    cr.save();
    cr.translate(position.x, position.y);
    cr.rectangle(
        0.0, 0.0,
        button.size.width, button.size.height
    );
    cr.clip();

    let scale_factor = unsafe {
        c::eek_renderer_get_scale_factor(renderer)
    };
    let bounds = button.get_bounds();
    let (label_c, icon_name_c) = match &button.label {
        Label::Text(text) => (text.as_ptr(), ptr::null()),
        Label::IconName(name) => {
            let l = unsafe {
                // CStr doesn't allocate anything, so it only points to
                // the 'static str, avoiding a memory leak
                CStr::from_bytes_with_nul_unchecked(b"icon\0")
            };
            (l.as_ptr(), name.as_ptr())
        },
    };

    with_button_context(
        renderer,
        button,
        pressed,
        locked,
        |ctx| unsafe {
            // TODO: split into separate procedures:
            // draw outline, draw label, draw icon.
            c::eek_render_button_in_context(
                scale_factor,
                cairo::Context::to_raw_none(&cr),
                *ctx,
                bounds,
                icon_name_c,
                label_c,
            )
        }
    );

    cr.restore();
}

fn with_button_context<R, F: FnOnce(&c::GtkStyleContext) -> R>(
    renderer: c::EekRenderer,
    button: &Button,
    pressed: keyboard::PressType,
    locked: bool,
    operation: F,
) -> R {
    let outline_name_c = button.outline_name.as_ptr();
    
    let ctx = unsafe {
        c::eek_get_style_context_for_button(
            renderer,
            button.name.as_ptr(),
            outline_name_c,
            pressed as u64,
            locked as u64,
        )
    };
    
    let r = operation(&ctx);

    unsafe {
        c::eek_put_style_context_for_button(
            ctx,
            outline_name_c,
            locked as u64,
        )
    };

    r
}

pub fn queue_redraw(keyboard: EekGtkKeyboard) {
    let widget = unsafe { gtk::Widget::from_glib_none(keyboard.0) };
    widget.queue_draw();
}
