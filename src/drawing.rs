/*! Drawing the UI */

use cairo;
use std::cell::RefCell;

use ::action::Action;
use ::keyboard;
use ::layout::{ Button, Layout };
use ::layout::c::{ EekGtkKeyboard, Point };
use ::submission::Submission;

use glib::translate::FromGlibPtrNone;
use gtk::WidgetExt;

mod c {
    use super::*;

    use cairo_sys;
    use std::os::raw::c_void;
    
    // This is constructed only in C, no need for warnings
    #[allow(dead_code)]
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    pub struct EekRenderer(*const c_void);

    extern "C" {
        // Button and View inside CButtonPlace are safe to pass to C
        // as long as they don't outlive the call
        // and nothing dereferences them
        #[allow(improper_ctypes)]
        pub fn eek_render_button(
            renderer: EekRenderer,
            cr: *mut cairo_sys::cairo_t,
            button: *const Button,
            pressed: u64,
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
    unsafe {
        c::eek_render_button(
            renderer,
            cairo::Context::to_raw_none(&cr),
            button as *const Button,
            pressed as u64,
            locked as u64,
        )
    };
    cr.restore();
}

pub fn queue_redraw(keyboard: EekGtkKeyboard) {
    let widget = unsafe { gtk::Widget::from_glib_none(keyboard.0) };
    widget.queue_draw();
}
