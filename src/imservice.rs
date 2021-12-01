/*! Manages zwp_input_method_v2 protocol.
 * 
 * Library module.
 */

use std::boxed::Box;
use std::ffi::CString;
use std::fmt;
use std::num::Wrapping;
use std::string::String;

use ::logging;
use ::util::c::into_cstring;

// Traits
use std::convert::TryFrom;
use ::logging::Warn;


/// Gathers stuff defined in C or called by C
pub mod c {
    use super::*;

    use std::os::raw::{c_char, c_void};

    pub use ::ui_manager::c::UIManager;
    pub use ::submission::c::StateManager;

    // The following defined in C
        
    /// struct zwp_input_method_v2*
    #[repr(transparent)]
    pub struct InputMethod(*const c_void);

    extern "C" {
        fn imservice_destroy_im(im: *mut c::InputMethod);

        #[allow(improper_ctypes)] // IMService will never be dereferenced in C
        pub fn imservice_connect_listeners(im: *mut InputMethod, imservice: *const IMService);
        pub fn eek_input_method_commit_string(im: *mut InputMethod, text: *const c_char);
        pub fn eek_input_method_delete_surrounding_text(im: *mut InputMethod, before: u32, after: u32);
        pub fn eek_input_method_commit(im: *mut InputMethod, serial: u32);
        fn eekboard_context_service_set_hint_purpose(state: *const StateManager, hint: u32, purpose: u32);
    }
    
    // The following defined in Rust. TODO: wrap naked pointers to Rust data inside RefCells to prevent multiple writers
    
    // TODO: is unsafe needed here?
    #[no_mangle]
    pub extern "C"
    fn imservice_handle_input_method_activate(imservice: *mut IMService,
        im: *const InputMethod)
    {
        let imservice = check_imservice(imservice, im).unwrap();
        imservice.preedit_string = String::new();
        imservice.pending = IMProtocolState {
            active: true,
            ..IMProtocolState::default()
        };
    }
    
    #[no_mangle]
    pub extern "C"
    fn imservice_handle_input_method_deactivate(imservice: *mut IMService,
        im: *const InputMethod)
    {
        let imservice = check_imservice(imservice, im).unwrap();
        imservice.pending = IMProtocolState {
            active: false,
            ..imservice.pending.clone()
        };
    }
    
    #[no_mangle]
    pub extern "C"
    fn imservice_handle_surrounding_text(imservice: *mut IMService,
        im: *const InputMethod,
        text: *const c_char, cursor: u32, _anchor: u32)
    {
        let imservice = check_imservice(imservice, im).unwrap();
        imservice.pending = IMProtocolState {
            surrounding_text: into_cstring(text)
                .expect("Received invalid string")
                .expect("Received null string"),
            surrounding_cursor: cursor,
            ..imservice.pending.clone()
        };
    }
    
    #[no_mangle]
    pub extern "C"
    fn imservice_handle_content_type(imservice: *mut IMService,
        im: *const InputMethod,
        hint: u32, purpose: u32)
    {
        let imservice = check_imservice(imservice, im).unwrap();
        imservice.pending = IMProtocolState {
            content_hint: {
                ContentHint::from_bits(hint)
                    .or_print(
                        logging::Problem::Warning,
                        "Received invalid hint flags",
                    )
                    .unwrap_or(ContentHint::NONE)
            },
            content_purpose: {
                ContentPurpose::try_from(purpose)
                    .or_print(
                        logging::Problem::Warning,
                        "Received invalid purpose value",
                    )
                    .unwrap_or(ContentPurpose::Normal)
            },
            ..imservice.pending.clone()
        };
    }
    
    #[no_mangle]
    pub extern "C"
    fn imservice_handle_text_change_cause(imservice: *mut IMService,
        im: *const InputMethod,
        cause: u32)
    {
        let imservice = check_imservice(imservice, im).unwrap();
        imservice.pending = IMProtocolState {
            text_change_cause: {
                ChangeCause::try_from(cause)
                    .or_print(
                        logging::Problem::Warning,
                        "Received invalid cause value",
                    )
                    .unwrap_or(ChangeCause::InputMethod)
            },
            ..imservice.pending.clone()
        };
    }
    
    #[no_mangle]
    pub extern "C"
    fn imservice_handle_done(imservice: *mut IMService,
        im: *const InputMethod)
    {
        let imservice = check_imservice(imservice, im).unwrap();
        let active_changed = imservice.current.active ^ imservice.pending.active;

        imservice.current = imservice.pending.clone();
        imservice.pending = IMProtocolState {
            active: imservice.current.active,
            ..IMProtocolState::default()
        };

        imservice.serial += Wrapping(1u32);

        if active_changed {
            (imservice.active_callback)(imservice.current.active);
        }
        if imservice.current.active {
            unsafe {
                eekboard_context_service_set_hint_purpose(
                    imservice.state_manager,
                    imservice.current.content_hint.bits(),
                    imservice.current.content_purpose.clone() as u32,
                );
            }
        }
    }
    
    // TODO: this is really untested
    #[no_mangle]
    pub extern "C"
    fn imservice_handle_unavailable(imservice: *mut IMService,
        im: *mut InputMethod)
    {
        let imservice = check_imservice(imservice, im).unwrap();
        unsafe { imservice_destroy_im(im); }

        // no need to care about proper double-buffering,
        // the keyboard is already decommissioned
        imservice.current.active = false;

        (imservice.active_callback)(imservice.current.active);
    }    

    // FIXME: destroy and deallocate
    
    // Helpers
    
    /// Convenience method for referencing the IMService raw pointer,
    /// and for verifying that the input method passed along
    /// matches the one in the `imservice`.
    ///
    /// The lifetime of the returned value is 'static,
    /// due to the fact that the lifetime of a raw pointer is undefined.
    /// Care must be take
    /// not to exceed the lifetime of the pointer with the reference,
    /// especially not to store it.
    fn check_imservice(imservice: *mut IMService, im: *const InputMethod)
        -> Result<&'static mut IMService, &'static str>
    {
        if imservice.is_null() {
            return Err("Null imservice pointer");
        }
        let imservice: &mut IMService = unsafe { &mut *imservice };
        if im == imservice.im {
            Ok(imservice)
        } else {
            Err("Imservice doesn't contain the input method")
        }
    }
}


bitflags!{
    /// Map to `text_input_unstable_v3.content_hint` values
    pub struct ContentHint: u32 {
        const NONE = 0x0;
        const COMPLETION = 0x1;
        const SPELLCHECK = 0x2;
        const AUTO_CAPITALIZATION = 0x4;
        const LOWERCASE = 0x8;
        const UPPERCASE = 0x10;
        const TITLECASE = 0x20;
        const HIDDEN_TEXT = 0x40;
        const SENSITIVE_DATA = 0x80;
        const LATIN = 0x100;
        const MULTILINE = 0x200;
    }
}

/// Map to `text_input_unstable_v3.content_purpose` values
///
/// ```
/// use rs::imservice::ContentPurpose;
/// assert_eq!(ContentPurpose::Alpha as u32, 1);
/// ```
#[derive(Debug, Copy, Clone)]
pub enum ContentPurpose {
    Normal = 0,
    Alpha = 1,
    Digits = 2,
    Number = 3,
    Phone = 4,
    Url = 5,
    Email = 6,
    Name = 7,
    Password = 8,
    Pin = 9,
    Date = 10,
    Time = 11,
    Datetime = 12,
    Terminal = 13,
}

// Utilities from ::logging need a printable error type
pub struct UnrecognizedValue;

impl fmt::Display for UnrecognizedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unrecognized value")
    }
}

impl TryFrom<u32> for ContentPurpose {
    type Error = UnrecognizedValue;
    fn try_from(num: u32) -> Result<Self, Self::Error> {
        use self::ContentPurpose::*;
        match num {
            0 => Ok(Normal),
            1 => Ok(Alpha),
            2 => Ok(Digits),
            3 => Ok(Number),
            4 => Ok(Phone),
            5 => Ok(Url),
            6 => Ok(Email),
            7 => Ok(Name),
            8 => Ok(Password),
            9 => Ok(Pin),
            10 => Ok(Date),
            11 => Ok(Time),
            12 => Ok(Datetime),
            13 => Ok(Terminal),
            _ => Err(UnrecognizedValue),
        }
    }
}

/// Map to `text_input_unstable_v3.change_cause` values
#[derive(Debug, Clone)]
pub enum ChangeCause {
    InputMethod = 0,
    Other = 1,
}

impl TryFrom<u32> for ChangeCause {
    type Error = UnrecognizedValue;
    fn try_from(num: u32) -> Result<Self, Self::Error> {
        match num {
            0 => Ok(ChangeCause::InputMethod),
            1 => Ok(ChangeCause::Other),
            _ => Err(UnrecognizedValue)
        }
    }
}

/// Describes the desired state of the input method as requested by the server
#[derive(Clone)]
struct IMProtocolState {
    surrounding_text: CString,
    surrounding_cursor: u32,
    content_purpose: ContentPurpose,
    content_hint: ContentHint,
    text_change_cause: ChangeCause,
    active: bool,
}

impl Default for IMProtocolState {
    fn default() -> IMProtocolState {
        IMProtocolState {
            surrounding_text: CString::default(),
            surrounding_cursor: 0, // TODO: mark that there's no cursor
            content_hint: ContentHint::NONE,
            content_purpose: ContentPurpose::Normal,
            text_change_cause: ChangeCause::InputMethod,
            active: false,
        }
    }
}

pub struct IMService {
    /// Owned reference (still created and destroyed in C)
    pub im: *mut c::InputMethod,
    /// Unowned reference. Be careful, it's shared with C at large
    state_manager: *const c::StateManager,
    active_callback: Box<dyn Fn(bool)>,

    pending: IMProtocolState,
    current: IMProtocolState, // turn current into an idiomatic representation?
    preedit_string: String,
    serial: Wrapping<u32>,
}

pub enum SubmitError {
    /// The input method had not been activated
    NotActive,
}

impl IMService {
    pub fn new(
        im: *mut c::InputMethod,
        state_manager: *const c::StateManager,
        active_callback: Box<dyn Fn(bool)>,
    ) -> Box<IMService> {
        // IMService will be referenced to by C,
        // so it needs to stay in the same place in memory via Box
        let imservice = Box::new(IMService {
            im,
            active_callback,
            state_manager,
            pending: IMProtocolState::default(),
            current: IMProtocolState::default(),
            preedit_string: String::new(),
            serial: Wrapping(0u32),
        });
        unsafe {
            c::imservice_connect_listeners(
                im,
                imservice.as_ref() as *const IMService,
            );
        }
        imservice
    }

    pub fn commit_string(&self, text: &CString) -> Result<(), SubmitError> {
        match self.current.active {
            true => {
                unsafe {
                    c::eek_input_method_commit_string(self.im, text.as_ptr())
                }
                Ok(())
            },
            false => Err(SubmitError::NotActive),
        }
    }

    pub fn delete_surrounding_text(
        &self,
        before: u32, after: u32,
    ) -> Result<(), SubmitError> {
        match self.current.active {
            true => {
                unsafe {
                    c::eek_input_method_delete_surrounding_text(
                        self.im,
                        before, after,
                    )
                }
                Ok(())
            },
            false => Err(SubmitError::NotActive),
        }
    }

    pub fn commit(&mut self) -> Result<(), SubmitError> {
        match self.current.active {
            true => {
                unsafe {
                    c::eek_input_method_commit(self.im, self.serial.0)
                }
                Ok(())
            },
            false => Err(SubmitError::NotActive),
        }
    }

    pub fn is_active(&self) -> bool {
        self.current.active
    }
}
