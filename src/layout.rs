/*!
 * Layout-related data.
 * 
 * The `View` contains `Row`s and each `Row` contains `Button`s.
 * They carry data relevant to their positioning only,
 * except the Button, which also carries some data
 * about its appearance and function.
 * 
 * The layout is determined bottom-up, by measuring `Button` sizes,
 * deriving `Row` sizes from them, and then centering them within the `View`.
 * 
 * That makes the `View` position immutable,
 * and therefore different than the other positions.
 * 
 * Note that it might be a better idea
 * to make `View` position depend on its contents,
 * and let the renderer scale and center it within the widget.
 */

use std::cell::RefCell;
use std::collections::{ HashMap, HashSet };
use std::ffi::CString;
use std::fmt;
use std::rc::Rc;
use std::vec::Vec;

use ::action::Action;
use ::drawing;
use ::keyboard::KeyState;
use ::logging;
use ::manager;
use ::submission::{ Submission, SubmitData, Timestamp };
use ::util::find_max_double;

// Traits
use std::borrow::Borrow;
use ::logging::Warn;

/// Gathers stuff defined in C or called by C
pub mod c {
    use super::*;

    use gtk_sys;
    use std::ffi::CStr;
    use std::os::raw::{ c_char, c_void };
    use std::ptr;

    use std::ops::{ Add, Sub };

    // The following defined in C
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    pub struct EekGtkKeyboard(pub *const gtk_sys::GtkWidget);

    extern "C" {
        #[allow(improper_ctypes)]
        pub fn eek_gtk_keyboard_emit_feedback(
            keyboard: EekGtkKeyboard,
        );
    }

    /// Defined in eek-types.h
    #[repr(C)]
    #[derive(Clone, Debug, PartialEq)]
    pub struct Point {
        pub x: f64,
        pub y: f64,
    }
    
    impl Add for Point {
        type Output = Self;
        fn add(self, other: Self) -> Self {
            &self + other
        }
    }

    impl Add<Point> for &Point {
        type Output = Point;
        fn add(self, other: Point) -> Point {
            Point {
                x: self.x + other.x,
                y: self.y + other.y,
            }
        }
    }
    
    impl Sub<&Point> for Point {
        type Output = Point;
        fn sub(self, other: &Point) -> Point {
            Point {
                x: self.x - other.x,
                y: self.y - other.y,
            }
        }
    }

    /// Defined in eek-types.h
    #[repr(C)]
    #[derive(Clone, Debug, PartialEq)]
    pub struct Bounds {
        pub x: f64,
        pub y: f64,
        pub width: f64,
        pub height: f64
    }

    impl Bounds {
        pub fn contains(&self, point: &Point) -> bool {
            point.x > self.x && point.x < self.x + self.width
                && point.y > self.y && point.y < self.y + self.height
        }
    }

    /// Translate and then scale
    #[repr(C)]
    pub struct Transformation {
        pub origin_x: f64,
        pub origin_y: f64,
        pub scale: f64,
    }

    impl Transformation {
        /// Applies the new transformation after this one
        pub fn chain(self, next: Transformation) -> Transformation {
            Transformation {
                origin_x: self.origin_x + self.scale * next.origin_x,
                origin_y: self.origin_y + self.scale * next.origin_y,
                scale: self.scale * next.scale,
            }
        }
        fn forward(&self, p: Point) -> Point {
            Point {
                x: (p.x - self.origin_x) / self.scale,
                y: (p.y - self.origin_y) / self.scale,
            }
        }
        fn reverse(&self, p: Point) -> Point {
            Point {
                x: p.x * self.scale + self.origin_x,
                y: p.y * self.scale + self.origin_y,
            }
        }
        pub fn reverse_bounds(&self, b: Bounds) -> Bounds {
            let start = self.reverse(Point { x: b.x, y: b.y });
            let end = self.reverse(Point {
                x: b.x + b.width,
                y: b.y + b.height,
            });
            Bounds {
                x: start.x,
                y: start.y,
                width: end.x - start.x,
                height: end.y - start.y,
            }
        }
    }
    
    // This is constructed only in C, no need for warnings
    #[allow(dead_code)]
    #[repr(transparent)]
    pub struct LevelKeyboard(*const c_void);

    // The following defined in Rust. TODO: wrap naked pointers to Rust data inside RefCells to prevent multiple writers

    #[no_mangle]
    pub extern "C"
    fn squeek_button_get_bounds(button: *const ::layout::Button) -> Bounds {
        let button = unsafe { &*button };
        Bounds {
            x: 0.0, y: 0.0,
            width: button.size.width, height: button.size.height
        }
    }

    #[no_mangle]
    pub extern "C"
    fn squeek_button_get_label(
        button: *const ::layout::Button
    ) -> *const c_char {
        let button = unsafe { &*button };
        match &button.label {
            Label::Text(text) => text.as_ptr(),
            // returning static strings to C is a bit cumbersome
            Label::IconName(_) => unsafe {
                // CStr doesn't allocate anything, so it only points to
                // the 'static str, avoiding a memory leak
                CStr::from_bytes_with_nul_unchecked(b"icon\0")
            }.as_ptr(),
        }
    }
    
    #[no_mangle]
    pub extern "C"
    fn squeek_button_get_icon_name(button: *const Button) -> *const c_char {
        let button = unsafe { &*button };
        match &button.label {
            Label::Text(_) => ptr::null(),
            Label::IconName(name) => name.as_ptr(),
        }
    }
    
    #[no_mangle]
    pub extern "C"
    fn squeek_button_get_name(button: *const Button) -> *const c_char {
        let button = unsafe { &*button };
        button.name.as_ptr()
    }
    
    #[no_mangle]
    pub extern "C"
    fn squeek_button_get_outline_name(button: *const Button) -> *const c_char {
        let button = unsafe { &*button };
        button.outline_name.as_ptr()
    }
    
    #[no_mangle]
    pub extern "C"
    fn squeek_button_print(button: *const ::layout::Button) {
        let button = unsafe { &*button };
        println!("{:?}", button);
    }
    
    /// Positions the layout contents within the available space.
    /// The origin of the transformation is the point inside the margins.
    #[no_mangle]
    pub extern "C"
    fn squeek_layout_calculate_transformation(
        layout: *const Layout,
        allocation_width: f64,
        allocation_height: f64,
    ) -> Transformation {
        let layout = unsafe { &*layout };
        layout.calculate_transformation(Size {
            width: allocation_width,
            height: allocation_height,
        })
    }

    #[no_mangle]
    pub extern "C"
    fn squeek_layout_get_kind(layout: *const Layout) -> u32 {
        let layout = unsafe { &*layout };
        layout.kind.clone() as u32
    }

    #[no_mangle]
    pub extern "C"
    fn squeek_layout_free(layout: *mut Layout) {
        unsafe { Box::from_raw(layout) };
    }

    /// Entry points for more complex procedures and algorithms which span multiple modules
    pub mod procedures {
        use super::*;

        /// Release pointer in the specified position
        #[no_mangle]
        pub extern "C"
        fn squeek_layout_release(
            layout: *mut Layout,
            submission: *mut Submission,
            widget_to_layout: Transformation,
            time: u32,
            manager: manager::c::Manager,
            ui_keyboard: EekGtkKeyboard,
        ) {
            let time = Timestamp(time);
            let layout = unsafe { &mut *layout };
            let submission = unsafe { &mut *submission };
            let ui_backend = UIBackend {
                widget_to_layout,
                keyboard: ui_keyboard,
            };

            // The list must be copied,
            // because it will be mutated in the loop
            for key in layout.pressed_keys.clone() {
                let key: &Rc<RefCell<KeyState>> = key.borrow();
                seat::handle_release_key(
                    layout,
                    submission,
                    Some(&ui_backend),
                    time,
                    Some(manager),
                    key,
                );
            }
            drawing::queue_redraw(ui_keyboard);
        }

        /// Release all buttons but don't redraw
        #[no_mangle]
        pub extern "C"
        fn squeek_layout_release_all_only(
            layout: *mut Layout,
            submission: *mut Submission,
            time: u32,
        ) {
            let layout = unsafe { &mut *layout };
            let submission = unsafe { &mut *submission };
            // The list must be copied,
            // because it will be mutated in the loop
            for key in layout.pressed_keys.clone() {
                let key: &Rc<RefCell<KeyState>> = key.borrow();
                seat::handle_release_key(
                    layout,
                    submission,
                    None, // don't update UI
                    Timestamp(time),
                    None, // don't switch layouts
                    &mut key.clone(),
                );
            }
        }

        #[no_mangle]
        pub extern "C"
        fn squeek_layout_depress(
            layout: *mut Layout,
            submission: *mut Submission,
            x_widget: f64, y_widget: f64,
            widget_to_layout: Transformation,
            time: u32,
            ui_keyboard: EekGtkKeyboard,
        ) {
            let layout = unsafe { &mut *layout };
            let submission = unsafe { &mut *submission };
            let point = widget_to_layout.forward(
                Point { x: x_widget, y: y_widget }
            );

            let state = layout.find_button_by_position(point)
                .map(|place| place.button.state.clone());
            
            if let Some(state) = state {
                seat::handle_press_key(
                    layout,
                    submission,
                    Timestamp(time),
                    &state,
                );
                // maybe TODO: draw on the display buffer here
                drawing::queue_redraw(ui_keyboard);
                unsafe {
                    eek_gtk_keyboard_emit_feedback(ui_keyboard);
                }
            };
        }

        // FIXME: this will work funny
        // when 2 touch points are on buttons and moving one after another
        // Solution is to have separate pressed lists for each point
        #[no_mangle]
        pub extern "C"
        fn squeek_layout_drag(
            layout: *mut Layout,
            submission: *mut Submission,
            x_widget: f64, y_widget: f64,
            widget_to_layout: Transformation,
            time: u32,
            manager: manager::c::Manager,
            ui_keyboard: EekGtkKeyboard,
        ) {
            let time = Timestamp(time);
            let layout = unsafe { &mut *layout };
            let submission = unsafe { &mut *submission };
            let ui_backend = UIBackend {
                widget_to_layout,
                keyboard: ui_keyboard,
            };
            let point = ui_backend.widget_to_layout.forward(
                Point { x: x_widget, y: y_widget }
            );
            
            let pressed = layout.pressed_keys.clone();
            let button_info = {
                let place = layout.find_button_by_position(point);
                place.map(|place| {(
                    place.button.state.clone(),
                    place.button.clone(),
                    place.offset,
                )})
            };

            if let Some((state, _button, _view_position)) = button_info {
                let mut found = false;
                for wrapped_key in pressed {
                    let key: &Rc<RefCell<KeyState>> = wrapped_key.borrow();
                    if Rc::ptr_eq(&state, &wrapped_key.0) {
                        found = true;
                    } else {
                        seat::handle_release_key(
                            layout,
                            submission,
                            Some(&ui_backend),
                            time,
                            Some(manager),
                            key,
                        );
                    }
                }
                if !found {
                    seat::handle_press_key(
                        layout,
                        submission,
                        time,
                        &state,
                    );
                    // maybe TODO: draw on the display buffer here
                    unsafe {
                        eek_gtk_keyboard_emit_feedback(ui_keyboard);
                    }
                }
            } else {
                for wrapped_key in pressed {
                    let key: &Rc<RefCell<KeyState>> = wrapped_key.borrow();
                    seat::handle_release_key(
                        layout,
                        submission,
                        Some(&ui_backend),
                        time,
                        Some(manager),
                        key,
                    );
                }
            }
            drawing::queue_redraw(ui_keyboard);
        }

        #[cfg(test)]
        mod test {
            use super::*;
            
            fn near(a: f64, b: f64) -> bool {
                (a - b).abs() < ((a + b) * 0.001f64).abs()
            }
            
            #[test]
            fn transform_back() {
                let transform = Transformation {
                    origin_x: 10f64,
                    origin_y: 11f64,
                    scale: 12f64,
                };
                let point = Point { x: 1f64, y: 1f64 };
                let transformed = transform.reverse(transform.forward(point.clone()));
                assert!(near(point.x, transformed.x));
                assert!(near(point.y, transformed.y));
            }
        }
    }
}

pub struct ButtonPlace<'a> {
    button: &'a Button,
    offset: c::Point,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Label {
    /// Text used to display the symbol
    Text(CString),
    /// Icon name used to render the symbol
    IconName(CString),
}

/// The graphical representation of a button
#[derive(Clone, Debug)]
pub struct Button {
    /// ID string, e.g. for CSS 
    pub name: CString,
    /// Label to display to the user
    pub label: Label,
    pub size: Size,
    /// The name of the visual class applied
    pub outline_name: CString,
    /// current state, shared with other buttons
    pub state: Rc<RefCell<KeyState>>,
}

/// The graphical representation of a row of buttons
#[derive(Clone, Debug)]
pub struct Row {
    /// Buttons together with their offset from the left relative to the row.
    /// ie. the first button always start at 0.
    buttons: Vec<(f64, Box<Button>)>,

    /// Total size of the row
    size: Size,
}

impl Row {
    pub fn new(buttons: Vec<(f64, Box<Button>)>) -> Row {
        // Make sure buttons are sorted by offset.
        debug_assert!({
            let mut sorted = buttons.clone();
            sorted.sort_by(|(f1, _), (f2, _)| f1.partial_cmp(f2).unwrap());

            sorted.iter().map(|(f, _)| *f).collect::<Vec<_>>()
                == buttons.iter().map(|(f, _)| *f).collect::<Vec<_>>()
        });

        let width = buttons.iter().next_back()
            .map(|(x_offset, button)| button.size.width + x_offset)
            .unwrap_or(0.0);

        let height = find_max_double(
            buttons.iter(),
            |(_offset, button)| button.size.height,
        );

        Row { buttons, size: Size { width, height } }
    }

    pub fn get_size(&self) -> Size {
        self.size.clone()
    }

    pub fn get_buttons(&self) -> &Vec<(f64, Box<Button>)> {
        &self.buttons
    }

    /// Finds the first button that covers the specified point
    /// relative to row's position's origin
    fn find_button_by_position(&self, x: f64) -> &(f64, Box<Button>)
    {
        // Buttons are sorted so we can use a binary search to find the clicked
        // button. Note this doesn't check whether the point is actually within
        // a button. This is on purpose as we want a click past the left edge of
        // the left-most button to register as a click.
        let result = self.buttons.binary_search_by(
            |&(f, _)| f.partial_cmp(&x).unwrap()
        );

        let index = result.unwrap_or_else(|r| r);
        let index = if index > 0 { index - 1 } else { 0 };

        &self.buttons[index]
    }
}

#[derive(Clone, Debug)]
pub struct Spacing {
    pub row: f64,
    pub button: f64,
}

pub struct View {
    /// Rows together with their offsets from the top left
    rows: Vec<(c::Point, Row)>,

    /// Total size of the view
    size: Size,
}

impl View {
    pub fn new(rows: Vec<(f64, Row)>) -> View {
        // Make sure rows are sorted by offset.
        debug_assert!({
            let mut sorted = rows.clone();
            sorted.sort_by(|(f1, _), (f2, _)| f1.partial_cmp(f2).unwrap());

            sorted.iter().map(|(f, _)| *f).collect::<Vec<_>>()
                == rows.iter().map(|(f, _)| *f).collect::<Vec<_>>()
        });

        // No need to call `get_rows()`,
        // as the biggest row is the most far reaching in both directions
        // because they are all centered.
        let width = find_max_double(rows.iter(), |(_offset, row)| row.size.width);

        let height = rows.iter().next_back()
            .map(|(y_offset, row)| row.size.height + y_offset)
            .unwrap_or(0.0);

        // Center the rows
        let rows = rows.into_iter().map(|(y_offset, row)| {(
                c::Point {
                    x: (width - row.size.width) / 2.0,
                    y: y_offset,
                },
                row,
            )}).collect::<Vec<_>>();

        View { rows, size: Size { width, height } }
    }
    /// Finds the first button that covers the specified point
    /// relative to view's position's origin
    fn find_button_by_position(&self, point: c::Point)
        -> Option<ButtonPlace>
    {
        // Only test bounds of the view here, letting rows/column search extend
        // to the edges of these bounds.
        let bounds = c::Bounds {
            x: 0.0,
            y: 0.0,
            width: self.size.width,
            height: self.size.height,
        };
        if !bounds.contains(&point) {
            return None;
        }

        // Rows are sorted so we can use a binary search to find the row.
        let result = self.rows.binary_search_by(
            |(f, _)| f.y.partial_cmp(&point.y).unwrap()
        );

        let index = result.unwrap_or_else(|r| r);
        let index = if index > 0 { index - 1 } else { 0 };

        let row = &self.rows[index];
        let button = row.1.find_button_by_position(point.x - row.0.x);

        Some(ButtonPlace {
            button: &button.1,
            offset: &row.0 + c::Point { x: button.0, y: 0.0 },
        })
    }
    
    pub fn get_size(&self) -> Size {
        self.size.clone()
    }
    
    /// Returns positioned rows, with appropriate x offsets (centered)
    pub fn get_rows(&self) -> &Vec<(c::Point, Row)> {
        &self.rows
    }

    /// Returns a size which contains all the views
    /// if they are all centered on the same point.
    pub fn calculate_super_size(views: Vec<&View>) -> Size {
        Size {
            height: find_max_double(
                views.iter(),
                |view| view.size.height,
            ),
            width: find_max_double(
                views.iter(),
                |view| view.size.width,
            ),
        }
    }
}

/// The physical characteristic of layout for the purpose of styling
#[derive(Clone, PartialEq, Debug)]
pub enum ArrangementKind {
    Base = 0,
    Wide = 1,
}

#[derive(Debug, PartialEq)]
pub struct Margins {
    pub top: f64,
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
}

// TODO: split into sth like
// Arrangement (views) + details (keymap) + State (keys)
/// State of the UI, contains the backend as well
pub struct Layout {
    pub margins: Margins,
    pub kind: ArrangementKind,
    pub current_view: String,
    // Views own the actual buttons which have state
    // Maybe they should own UI only,
    // and keys should be owned by a dedicated non-UI-State?
    /// Point is the offset within the layout
    pub views: HashMap<String, (c::Point, View)>,

    // Non-UI stuff
    /// xkb keymaps applicable to the contained keys. Unchangeable
    pub keymaps: Vec<CString>,
    // Changeable state
    // a Vec would be enough, but who cares, this will be small & fast enough
    // TODO: turn those into per-input point *_buttons to track dragging.
    // The renderer doesn't need the list of pressed keys any more,
    // because it needs to iterate
    // through all buttons of the current view anyway.
    // When the list tracks actual location,
    // it becomes possible to place popovers and other UI accurately.
    pub pressed_keys: HashSet<::util::Pointer<RefCell<KeyState>>>,
}

/// A builder structure for picking up layout data from storage
pub struct LayoutData {
    /// Point is the offset within layout
    pub views: HashMap<String, (c::Point, View)>,
    pub keymaps: Vec<CString>,
    pub margins: Margins,
}

#[derive(Debug)]
struct NoSuchView;

impl fmt::Display for NoSuchView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "No such view")
    }
}

// Unfortunately, changes are not atomic due to mutability :(
// An error will not be recoverable
// The usage of &mut on Rc<RefCell<KeyState>> doesn't mean anything special.
// Cloning could also be used.
impl Layout {
    pub fn new(data: LayoutData, kind: ArrangementKind) -> Layout {
        Layout {
            kind,
            current_view: "base".to_owned(),
            views: data.views,
            keymaps: data.keymaps,
            pressed_keys: HashSet::new(),
            margins: data.margins,
        }
    }

    pub fn get_current_view_position(&self) -> &(c::Point, View) {
        &self.views
            .get(&self.current_view).expect("Selected nonexistent view")
    }

    pub fn get_current_view(&self) -> &View {
        &self.views.get(&self.current_view).expect("Selected nonexistent view").1
    }

    fn set_view(&mut self, view: String) -> Result<(), NoSuchView> {
        if self.views.contains_key(&view) {
            self.current_view = view;
            Ok(())
        } else {
            Err(NoSuchView)
        }
    }

    /// Calculates size without margins
    fn calculate_inner_size(&self) -> Size {
        View::calculate_super_size(
            self.views.iter().map(|(_, (_offset, v))| v).collect()
        )
    }

    /// Size including margins
    fn calculate_size(&self) -> Size {
        let inner_size = self.calculate_inner_size();
        Size {
            width: self.margins.left + inner_size.width + self.margins.right,
            height: (
                self.margins.top
                + inner_size.height
                + self.margins.bottom
            ),
        }
    }
    
    pub fn calculate_transformation(
        &self,
        available: Size,
    ) -> c::Transformation {
        let size = self.calculate_size();
        let h_scale = available.width / size.width;
        let v_scale = available.height / size.height;
        let scale = if h_scale < v_scale { h_scale } else { v_scale };
        let outside_margins = c::Transformation {
            origin_x: (available.width - (scale * size.width)) / 2.0,
            origin_y: (available.height - (scale * size.height)) / 2.0,
            scale: scale,
        };
        outside_margins.chain(c::Transformation {
            origin_x: self.margins.left,
            origin_y: self.margins.top,
            scale: 1.0,
        })
    }

    fn find_button_by_position(&self, point: c::Point) -> Option<ButtonPlace> {
        let (offset, layout) = self.get_current_view_position();
        layout.find_button_by_position(point - offset)
    }

    pub fn foreach_visible_button<F>(&self, mut f: F)
        where F: FnMut(c::Point, &Box<Button>)
    {
        let (view_offset, view) = self.get_current_view_position();
        for (row_offset, row) in view.get_rows() {
            for (x_offset, button) in &row.buttons {
                let offset = view_offset
                    + row_offset.clone()
                    + c::Point { x: *x_offset, y: 0.0 };
                f(offset, button);
            }
        }
    }

    pub fn get_locked_keys(&self) -> Vec<Rc<RefCell<KeyState>>> {
        let mut out = Vec::new();
        let view = self.get_current_view();
        for (_, row) in view.get_rows() {
            for (_, button) in &row.buttons {
                let locked = {
                    let state = RefCell::borrow(&button.state).clone();
                    state.action.is_locked(&self.current_view)
                };
                if locked {
                    out.push(button.state.clone());
                }
            }
        }
        out
    }
}

mod procedures {
    use super::*;

    type Place<'v> = (c::Point, &'v Box<Button>);

    /// Finds all buttons referring to the key in `state`,
    /// together with their offsets within the view.
    pub fn find_key_places<'v, 's>(
        view: &'v View,
        state: &'s Rc<RefCell<KeyState>>
    ) -> Vec<Place<'v>> {
        view.get_rows().iter().flat_map(|(row_offset, row)| {
            row.buttons.iter()
                .filter_map(move |(x_offset, button)| {
                    if Rc::ptr_eq(&button.state, state) {
                        Some((
                            row_offset + c::Point { x: *x_offset, y: 0.0 },
                            button,
                        ))
                    } else {
                        None
                    }
                })
        }).collect()
    }
    
    #[cfg(test)]
    mod test {
        use super::*;

        use ::layout::test::*;

        /// Checks whether the path points to the same boxed instances.
        /// The instance constraint will be droppable
        /// when C stops holding references to the data
        #[test]
        fn view_has_button() {
            fn as_ptr<T>(v: &Box<T>) -> *const T {
                v.as_ref() as *const T
            }

            let state = make_state();
            let state_clone = state.clone();

            let button = make_button_with_state("1".into(), state);
            let button_ptr = as_ptr(&button);

            let row = Row::new(vec!((0.1, button)));

            let view = View::new(vec!((1.2, row)));

            assert_eq!(
                find_key_places(&view, &state_clone.clone()).into_iter()
                    .map(|(place, button)| { (place, as_ptr(button)) })
                    .collect::<Vec<_>>(),
                vec!(
                    (c::Point { x: 0.1, y: 1.2 }, button_ptr)
                )
            );

            let view = View::new(vec![]);
            assert_eq!(
                find_key_places(&view, &state_clone.clone()).is_empty(),
                true
            );
        }
    }
}

pub struct UIBackend {
    widget_to_layout: c::Transformation,
    keyboard: c::EekGtkKeyboard,
}

/// Top level procedures, dispatching to everything
mod seat {
    use super::*;

    fn try_set_view(layout: &mut Layout, view_name: String) {
        layout.set_view(view_name.clone())
            .or_print(
                logging::Problem::Bug,
                &format!("Bad view {}, ignoring", view_name),
            );
    }

    /// A vessel holding an obligation to switch view.
    /// Use with #[must_use]
    struct ViewChange<'a> {
        layout: &'a mut Layout,
        view_name: Option<String>,
    }
    
    impl<'a> ViewChange<'a> {
        fn choose_view(self, view_name: String) -> ViewChange<'a> {
            ViewChange {
                view_name: Some(view_name),
                ..self
            }
        }
        fn apply(self) {
            if let Some(name) = self.view_name {
                try_set_view(self.layout, name);
            }
        }
    }

    /// Find all impermanent view changes and undo them in an arbitrary order.
    /// Return an obligation to actually switch the view.
    /// The final view is the "unlock" view
    /// from one of the currently stuck keys.
    // As long as only one stuck button is used, this should be fine.
    // This is guaranteed because pressing a lock button unlocks all others.
    // TODO: Make some broader guarantee about the resulting view,
    // e.g. by maintaining a stack of stuck keys.
    #[must_use]
    fn unstick_locks(layout: &mut Layout) -> ViewChange {
        let mut new_view = None;
        for key in layout.get_locked_keys().clone() {
            let key: &Rc<RefCell<KeyState>> = key.borrow();
            let key = RefCell::borrow(key);
            match &key.action {
                Action::LockView { lock: _, unlock: view } => {
                    new_view = Some(view.clone());
                },
                a => log_print!(
                    logging::Level::Bug,
                    "Non-locking action {:?} was found inside locked keys",
                    a,
                ),
            };
        }
        
        ViewChange {
            layout,
            view_name: new_view,
        }
    }

    pub fn handle_press_key(
        layout: &mut Layout,
        submission: &mut Submission,
        time: Timestamp,
        rckey: &Rc<RefCell<KeyState>>,
    ) {
        if !layout.pressed_keys.insert(::util::Pointer(rckey.clone())) {
            log_print!(
                logging::Level::Bug,
                "Key {:?} was already pressed", rckey,
            );
        }
        let key: KeyState = {
            RefCell::borrow(rckey).clone()
        };
        let action = key.action.clone();
        match action {
            Action::Submit {
                text: Some(text),
                keys: _,
            } => submission.handle_press(
                KeyState::get_id(rckey),
                SubmitData::Text(&text),
                &key.keycodes,
                time,
            ),
            Action::Submit {
                text: None,
                keys: _,
            } => submission.handle_press(
                KeyState::get_id(rckey),
                SubmitData::Keycodes,
                &key.keycodes,
                time,
            ),
            Action::Erase => submission.handle_press(
                KeyState::get_id(rckey),
                SubmitData::Erase,
                &key.keycodes,
                time,
            ),
            _ => {},
        };
        RefCell::replace(rckey, key.into_pressed());
    }

    pub fn handle_release_key(
        layout: &mut Layout,
        submission: &mut Submission,
        ui: Option<&UIBackend>,
        time: Timestamp,
        manager: Option<manager::c::Manager>,
        rckey: &Rc<RefCell<KeyState>>,
    ) {
        let key: KeyState = {
            RefCell::borrow(rckey).clone()
        };
        let action = key.action.clone();

        // update
        let key = key.into_released();

        // process changes
        match action {
            Action::Submit { text: _, keys: _ }
                | Action::Erase
            => {
                unstick_locks(layout).apply();
                submission.handle_release(KeyState::get_id(rckey), time);
            },
            Action::SetView(view) => {
                try_set_view(layout, view)
            },
            Action::LockView { lock, unlock } => {
                let gets_locked = !key.action.is_locked(&layout.current_view);
                unstick_locks(layout)
                    // It doesn't matter what the resulting view should be,
                    // it's getting changed anyway.
                    .choose_view(
                        match gets_locked {
                            true => lock.clone(),
                            false => unlock.clone(),
                        }
                    )
                    .apply()
            },
            Action::ApplyModifier(modifier) => {
                // FIXME: key id is unneeded with stateless locks
                let key_id = KeyState::get_id(rckey);
                let gets_locked = !submission.is_modifier_active(modifier.clone());
                match gets_locked {
                    true => submission.handle_add_modifier(
                        key_id,
                        modifier, time,
                    ),
                    false => submission.handle_drop_modifier(key_id, time),
                }
            }
            // only show when UI is present
            Action::ShowPreferences => if let Some(ui) = &ui {
                // only show when layout manager is available
                if let Some(manager) = manager {
                    let view = layout.get_current_view();
                    let places = ::layout::procedures::find_key_places(
                        view, &rckey,
                    );
                    // Getting first item will cause mispositioning
                    // with more than one button with the same key
                    // on the keyboard.
                    if let Some((position, button)) = places.get(0) {
                        let bounds = c::Bounds {
                            x: position.x,
                            y: position.y,
                            width: button.size.width,
                            height: button.size.height,
                        };
                        ::popover::show(
                            ui.keyboard,
                            ui.widget_to_layout.reverse_bounds(bounds),
                            manager,
                        );
                    }
                }
            },
        };

        let pointer = ::util::Pointer(rckey.clone());
        // Apply state changes
        layout.pressed_keys.remove(&pointer);
        // Commit activated button state changes
        RefCell::replace(rckey, key);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::ffi::CString;
    use ::keyboard::PressType;

    pub fn make_state() -> Rc<RefCell<::keyboard::KeyState>> {
        Rc::new(RefCell::new(::keyboard::KeyState {
            pressed: PressType::Released,
            keycodes: Vec::new(),
            action: Action::SetView("default".into()),
        }))
    }

    pub fn make_button_with_state(
        name: String,
        state: Rc<RefCell<::keyboard::KeyState>>,
    ) -> Box<Button> {
        Box::new(Button {
            name: CString::new(name.clone()).unwrap(),
            size: Size { width: 0f64, height: 0f64 },
            outline_name: CString::new("test").unwrap(),
            label: Label::Text(CString::new(name).unwrap()),
            state: state,
        })
    }
    
    #[test]
    fn check_centering() {
        //    A B
        // ---bar---
        let view = View::new(vec![
            (
                0.0,
                Row::new(vec![
                    (
                        0.0,
                        Box::new(Button {
                            size: Size { width: 5.0, height: 10.0 },
                            ..*make_button_with_state("A".into(), make_state())
                        }),
                    ),
                    (
                        5.0,
                        Box::new(Button {
                            size: Size { width: 5.0, height: 10.0 },
                            ..*make_button_with_state("B".into(), make_state())
                        }),
                    ),
                ]),
            ),
            (
                10.0,
                Row::new(vec![
                    (
                        0.0,
                        Box::new(Button {
                            size: Size { width: 30.0, height: 10.0 },
                            ..*make_button_with_state("bar".into(), make_state())
                        }),
                    ),
                ]),
            )
        ]);
        assert!(
            view.find_button_by_position(c::Point { x: 5.0, y: 5.0 })
                .unwrap().button.name.to_str().unwrap() == "A"
        );
        assert!(
            view.find_button_by_position(c::Point { x: 14.99, y: 5.0 })
                .unwrap().button.name.to_str().unwrap() == "A"
        );
        assert!(
            view.find_button_by_position(c::Point { x: 15.01, y: 5.0 })
                .unwrap().button.name.to_str().unwrap() == "B"
        );
        assert!(
            view.find_button_by_position(c::Point { x: 25.0, y: 5.0 })
                .unwrap().button.name.to_str().unwrap() == "B"
        );
    }

    #[test]
    fn check_bottom_margin() {
        // just one button
        let view = View::new(vec![
            (
                0.0,
                Row::new(vec![(
                    0.0,
                    Box::new(Button {
                        size: Size { width: 1.0, height: 1.0 },
                        ..*make_button_with_state("foo".into(), make_state())
                    }),
                )]),
            ),
        ]);
        let layout = Layout {
            current_view: String::new(),
            keymaps: Vec::new(),
            kind: ArrangementKind::Base,
            pressed_keys: HashSet::new(),
            // Lots of bottom margin
            margins: Margins {
                top: 0.0,
                left: 0.0,
                right: 0.0,
                bottom: 1.0,
            },
            views: hashmap! {
                String::new() => (c::Point { x: 0.0, y: 0.0 }, view),
            },
        };
        assert_eq!(
            layout.calculate_inner_size(),
            Size { width: 1.0, height: 1.0 }
        );
        assert_eq!(
            layout.calculate_size(),
            Size { width: 1.0, height: 2.0 }
        );
        // Don't change those values randomly!
        // They take advantage of incidental precise float representation
        // to even be comparable.
        let transformation = layout.calculate_transformation(
            Size { width: 2.0, height: 2.0 }
        );
        assert_eq!(transformation.scale, 1.0);
        assert_eq!(transformation.origin_x, 0.5);
        assert_eq!(transformation.origin_y, 0.0);
    }
}
