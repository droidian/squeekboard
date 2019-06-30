use std::boxed::Box;
use std::ffi::CString;
use std::num::Wrapping;
use std::string::String;


/// Gathers stuff defined in C or called by C
pub mod c {
    use super::*;
    
    use std::ffi::CStr;
    use std::os::raw::{c_char, c_void};
    
    fn into_cstring(s: *const c_char) -> Result<CString, std::ffi::NulError> {
        CString::new(
            unsafe {CStr::from_ptr(s)}.to_bytes()
        )
    }
    
    // The following defined in C
    
    /// struct zwp_input_method_v2*
    #[repr(transparent)]
    pub struct InputMethod(*const c_void);
    
    /// EekboardContextService*
    #[repr(transparent)]
    pub struct UIManager(*const c_void);
    
    #[no_mangle]
    extern "C" {
        fn imservice_make_visible(imservice: *const UIManager);
        fn imservice_try_hide(imservice: *const UIManager);
    }
    
    // The following defined in Rust. TODO: wrap naked pointers to Rust data inside RefCells to prevent multiple writers
    
    #[no_mangle]
    pub unsafe extern "C"
    fn imservice_new(im: *const InputMethod, ui_manager: *const UIManager) -> *mut IMService {
        Box::<IMService>::into_raw(Box::new(
            IMService {
                im: im,
                ui_manager: ui_manager,
                pending: IMProtocolState::default(),
                current: IMProtocolState::default(),
                preedit_string: String::new(),
                serial: Wrapping(0u32),
            }
        ))
    }
    
    // TODO: is unsafe needed here?
    #[no_mangle]
    pub unsafe extern "C"
    fn imservice_handle_input_method_activate(imservice: *mut IMService,
        _im: *const InputMethod)
    {
        let imservice = &mut *imservice;
        imservice.preedit_string = String::new();
        imservice.pending = IMProtocolState {
            active: true,
            ..IMProtocolState::default()
        };
    }
    
    #[no_mangle]
    pub unsafe extern "C"
    fn imservice_handle_input_method_deactivate(imservice: *mut IMService,
        _im: *const InputMethod)
    {
        let imservice = &mut *imservice;
        imservice.pending = IMProtocolState {
            active: false,
            ..imservice.pending.clone()
        };
    }
    
    #[no_mangle]
    pub unsafe extern "C"
    fn imservice_handle_surrounding_text(imservice: *mut IMService,
        _im: *const InputMethod,
        text: *const c_char, cursor: u32, _anchor: u32)
    {
        let imservice = &mut *imservice;
        imservice.pending = IMProtocolState {
            surrounding_text: into_cstring(text).expect("Received invalid string"),
            surrounding_cursor: cursor,
            ..imservice.pending
        };
    }
    
    #[no_mangle]
    pub unsafe extern "C"
    fn imservice_handle_commit_state(imservice: *mut IMService,
        _im: *const InputMethod)
    {
        let imservice = &mut *imservice;
        let active_changed = imservice.current.active ^ imservice.pending.active;
        
        imservice.serial += Wrapping(1u32);
        imservice.current = imservice.pending.clone();
        imservice.pending = IMProtocolState {
            active: imservice.current.active,
            ..IMProtocolState::default()
        };
        if active_changed {
            if imservice.current.active {
                imservice_make_visible(imservice.ui_manager);
            } else {
                imservice_try_hide(imservice.ui_manager);
            }
        }
    }
    
    // FIXME: destroy and deallocate
}

/// Describes the desired state of the input method as requested by the server
#[derive(Default, Clone)]
struct IMProtocolState {
    surrounding_text: CString,
    surrounding_cursor: u32,
    active: bool,
}

pub struct IMService {
    /// Owned reference (still created and destroyed in C)
    im: *const c::InputMethod,
    /// Unowned reference. Be careful, it's shared with C at large
    ui_manager: *const c::UIManager,

    pending: IMProtocolState,
    current: IMProtocolState, // turn current into an idiomatic representation?
    preedit_string: String,
    serial: Wrapping<u32>,
}
