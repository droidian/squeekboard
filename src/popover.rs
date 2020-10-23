/*! The layout chooser popover */

use gio;
use gtk;
use std::ffi::CString;
use std::cmp::Ordering;
use ::layout::c::{ Bounds, EekGtkKeyboard };
use ::locale;
use ::locale::{ OwnedTranslation, Translation, compare_current_locale };
use ::locale_config::system_locale;
use ::logging;
use ::manager;
use ::resources;

// Traits
use gio::ActionMapExt;
use gio::SettingsExt;
#[cfg(feature = "gio_v0_5")]
use gio::SimpleActionExt;
use glib::translate::FromGlibPtrNone;
use glib::variant::ToVariant;
#[cfg(not(feature = "gtk_v0_5"))]
use gtk::BuilderExtManual;
use gtk::PopoverExt;
use gtk::WidgetExt;
use std::io::Write;
use ::logging::Warn;

mod c {
    use std::os::raw::c_char;

    #[no_mangle]
    extern "C" {
        pub fn popover_open_settings_panel(panel: *const c_char);
    }
}

mod variants {
    use glib;
    use glib::Variant;
    use glib_sys;
    use std::os::raw::c_char;

    use glib::ToVariant;
    use glib::translate::FromGlibPtrFull;
    use glib::translate::FromGlibPtrNone;
    use glib::translate::ToGlibPtr;

    /// Unpacks tuple & array variants
    fn get_items(items: glib::Variant) -> Vec<glib::Variant> {
        let variant_naked = items.to_glib_none().0;
        let count = unsafe { glib_sys::g_variant_n_children(variant_naked) };
        (0..count).map(|index| 
            unsafe {
                glib::Variant::from_glib_full(
                    glib_sys::g_variant_get_child_value(variant_naked, index)
                )
            }
        ).collect()
    }

    /// Unpacks "a(ss)" variants
    pub fn get_tuples(items: glib::Variant) -> Vec<(String, String)> {
        get_items(items)
            .into_iter()
            .map(get_items)
            .map(|v| {
                (
                    v[0].get::<String>().unwrap(),
                    v[1].get::<String>().unwrap(),
                )
            })
            .collect()
    }

    /// "a(ss)" variant
    /// Rust doesn't allow implementing existing traits for existing types
    pub struct ArrayPairString(pub Vec<(String, String)>);
    
    impl ToVariant for ArrayPairString {
        fn to_variant(&self) -> Variant {
            let tspec = "a(ss)".to_glib_none();
            let builder = unsafe {
                let vtype = glib_sys::g_variant_type_checked_(tspec.0);
                glib_sys::g_variant_builder_new(vtype)
            };
            let ispec = "(ss)".to_glib_none();
            for (a, b) in &self.0 {
                let a = a.to_glib_none();
                let b = b.to_glib_none();
                // string pointers are weak references
                // and will get silently invalidated
                // as soon as the source is out of scope
                {
                    let a: *const c_char = a.0;
                    let b: *const c_char = b.0;
                    unsafe {
                        glib_sys::g_variant_builder_add(
                            builder,
                            ispec.0,
                            a, b
                        );
                    }
                }
            }
            unsafe {
                let ret = glib_sys::g_variant_builder_end(builder);
                glib_sys::g_variant_builder_unref(builder);
                glib::Variant::from_glib_none(ret)
            }
        }
    }
}

fn make_menu_builder(inputs: Vec<(&str, OwnedTranslation)>) -> gtk::Builder {
    let mut xml: Vec<u8> = Vec::new();
    writeln!(
        xml,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<interface>
  <menu id=\"app-menu\">
    <section>"
    ).unwrap();
    for (input_name, translation) in inputs {
        writeln!(
            xml,
            "
        <item>
            <attribute name=\"label\" translatable=\"yes\">{}</attribute>
            <attribute name=\"action\">layout</attribute>
            <attribute name=\"target\">{}</attribute>
        </item>",
            translation.0,
            input_name,
        ).unwrap();
    }
    writeln!(
        xml,
        "
    </section>
    <section>
        <item>
            <attribute name=\"label\" translatable=\"yes\">Keyboard Settings</attribute>
            <attribute name=\"action\">settings</attribute>
        </item>
    </section>
  </menu>
</interface>"
    ).unwrap();
    gtk::Builder::new_from_string(
        &String::from_utf8(xml).expect("Bad menu definition")
    )
}

fn get_settings(schema_name: &str) -> Option<gio::Settings> {
    let mut error_handler = logging::Print{};
    gio::SettingsSchemaSource::get_default()
        .or_warn(
            &mut error_handler,
            logging::Problem::Surprise,
            "No gsettings schemas installed.",
        )
        .and_then(|sss| 
            sss.lookup(schema_name, true)
                .or_warn(
                    &mut error_handler,
                    logging::Problem::Surprise,
                    &format!("Gsettings schema {} not installed", schema_name),
                )
        )
        .map(|_sschema| gio::Settings::new(schema_name))
}

fn set_layout(kind: String, name: String) {
    let settings = get_settings("org.gnome.desktop.input-sources");
    if let Some(settings) = settings {
        let inputs = settings.get_value("sources").unwrap();
        let current = (kind.clone(), name.clone());
        let inputs = variants::get_tuples(inputs).into_iter()
            .filter(|t| t != &current);
        let inputs = vec![(kind, name)].into_iter()
            .chain(inputs).collect();
        settings.set_value(
            "sources",
            &variants::ArrayPairString(inputs).to_variant(),
        );
        settings.apply();
    }
}

/// A reference to what the user wants to see
#[derive(PartialEq, Clone, Debug)]
enum LayoutId {
    /// Affects the layout in system settings
    System {
        kind: String,
        name: String,
    },
    /// Only affects what this input method presents
    Local(String),
}

impl LayoutId {
    fn get_name(&self) -> &str {
        match &self {
            LayoutId::System { kind: _, name } => name.as_str(),
            LayoutId::Local(name) => name.as_str(),
        }
    }
}

fn set_visible_layout(
    manager: manager::c::Manager,
    layout_id: LayoutId,
) {
    match layout_id {
        LayoutId::System { kind, name } => set_layout(kind, name),
        LayoutId::Local(name) => {
            let name = CString::new(name.as_str()).unwrap();
            let name_ptr = name.as_ptr();
            unsafe {
                manager::c::eekboard_context_service_set_overlay(
                    manager,
                    name_ptr,
                )
            }
        },
    }
}

/// Takes into account first any overlays, then system layouts from the list
fn get_current_layout(
    manager: manager::c::Manager,
    system_layouts: &Vec<LayoutId>,
) -> Option<LayoutId> {
    match manager::get_overlay(manager) {
        Some(name) => Some(LayoutId::Local(name)),
        None => system_layouts.get(0).map(LayoutId::clone),
    }
}

/// Translates all provided layout names according to current locale,
/// for the purpose of display (i.e. errors will be caught and reported)
fn translate_layout_names(layouts: &Vec<LayoutId>) -> Vec<OwnedTranslation> {
    // This procedure is rather ugly...
    // Xkb lookup *must not* be applied to non-system layouts,
    // so both translators can't be merged into one lookup table,
    // therefore must be done in two steps.
    // `XkbInfo` being temporary also means
    // that its return values must be copied,
    // forcing the use of `OwnedTranslation`.
    enum Status {
        /// xkb names should get all translated here
        Translated(OwnedTranslation),
        /// Builtin names need builtin translations
        Remaining(String),
    }

    // Attempt to take all xkb names from gnome-desktop's xkb info.
    let xkb_translator = locale::XkbInfo::new();

    let translated_names = layouts.iter()
        .map(|id| match id {
            LayoutId::System { name, kind: _ } => {
                xkb_translator.get_display_name(name)
                    .map(|s| Status::Translated(OwnedTranslation(s)))
                    .or_print(
                        logging::Problem::Surprise,
                        &format!("No display name for xkb layout {}", name),
                    ).unwrap_or_else(|| Status::Remaining(name.clone()))
            },
            LayoutId::Local(name) => Status::Remaining(name.clone()),
        });

    // Non-xkb layouts and weird xkb layouts
    // still need to be looked up in the internal database.
    let builtin_translations = system_locale()
        .map(|locale|
            locale.tags_for("messages")
                .next().unwrap() // guaranteed to exist
                .as_ref()
                .to_owned()
        )
        .or_print(logging::Problem::Surprise, "No locale detected")
        .and_then(|lang| {
            resources::get_layout_names(lang.as_str())
                .or_print(
                    logging::Problem::Surprise,
                    &format!("No translations for locale {}", lang),
                )
        });

    match builtin_translations {
        Some(translations) => {
            translated_names
                .map(|status| match status {
                    Status::Remaining(name) => {
                        translations.get(name.as_str())
                            .unwrap_or(&Translation(name.as_str()))
                            .to_owned()
                    },
                    Status::Translated(t) => t,
                })
                .collect()
        },
        None => {
            translated_names
                .map(|status| match status {
                    Status::Remaining(name) => OwnedTranslation(name),
                    Status::Translated(t) => t,
                })
                .collect()
        },
    }
}

pub fn show(
    window: EekGtkKeyboard,
    position: Bounds,
    manager: manager::c::Manager,
) {
    unsafe { gtk::set_initialized() };
    let window = unsafe { gtk::Widget::from_glib_none(window.0) };

    let overlay_layouts = resources::get_overlays().into_iter()
        .map(|name| LayoutId::Local(name.to_string()));

    let settings = get_settings("org.gnome.desktop.input-sources");
    let inputs = settings
        .map(|settings| {
            let inputs = settings.get_value("sources").unwrap();
            variants::get_tuples(inputs)
        })
        .unwrap_or_else(|| Vec::new());
    
    let system_layouts: Vec<LayoutId> = inputs.into_iter()
        .map(|(kind, name)| LayoutId::System { kind, name })
        .collect();

    let all_layouts: Vec<LayoutId> = system_layouts.clone()
        .into_iter()
        .chain(overlay_layouts)
        .collect();

    let translated_names = translate_layout_names(&all_layouts);
    
    // sorted collection of human and machine names
    let mut human_names: Vec<(OwnedTranslation, LayoutId)> = translated_names
        .into_iter()
        .zip(all_layouts.clone().into_iter())
        .collect();

    human_names.sort_unstable_by(|(tr_a, layout_a), (tr_b, layout_b)| {
        // Sort first by layout then name
        match (layout_a, layout_b) {
            (LayoutId::Local(_), LayoutId::System { .. }) => Ordering::Greater,
            (LayoutId::System { .. }, LayoutId::Local(_)) => Ordering::Less,
            _ => compare_current_locale(&tr_a.0, &tr_b.0)
        }
    });

    // GVariant doesn't natively support `enum`s,
    // so the `choices` vector will serve as a lookup table.
    let choices_with_translations: Vec<(String, (OwnedTranslation, LayoutId))>
        = human_names.into_iter()
            .enumerate()
                .map(|(i, human_entry)| {(
                    format!("{}_{}", i, human_entry.1.get_name()),
                    human_entry,
                )}).collect();


    let builder = make_menu_builder(
        choices_with_translations.iter()
            .map(|(id, (translation, _))| (id.as_str(), (*translation).clone()))
            .collect()
    );

    let choices: Vec<(String, LayoutId)>
        = choices_with_translations.into_iter()
            .map(|(id, (_tr, layout))| (id, layout))
            .collect();

    // Much more debuggable to populate the model & menu
    // from a string representation
    // than add items imperatively
    let model: gio::MenuModel = builder.get_object("app-menu").unwrap();

    let menu = gtk::Popover::new_from_model(Some(&window), &model);
    menu.set_pointing_to(&gtk::Rectangle {
        x: position.x.ceil() as i32,
        y: position.y.ceil() as i32,
        width: position.width.floor() as i32,
        height: position.width.floor() as i32,
    });

    if let Some(current_layout) = get_current_layout(manager, &system_layouts) {
        let current_name_variant = choices.iter()
            .find(
                |(_id, layout)| layout == &current_layout
            ).unwrap()
            .0.to_variant();

        let layout_action = gio::SimpleAction::new_stateful(
            "layout",
            Some(current_name_variant.type_()),
            &current_name_variant,
        );

        let menu_inner = menu.clone();
        layout_action.connect_change_state(move |_action, state| {
            match state {
                Some(v) => {
                    v.get::<String>()
                        .or_print(
                            logging::Problem::Bug,
                            &format!("Variant is not string: {:?}", v)
                        )
                        .map(|state| {
                            let (_id, layout) = choices.iter()
                                .find(
                                    |choices| state == choices.0
                                ).unwrap();
                            set_visible_layout(
                                manager,
                                layout.clone(),
                            )
                        });
                },
                None => log_print!(
                    logging::Level::Debug,
                    "No variant selected",
                ),
            };
            menu_inner.popdown();
        });

        let settings_action = gio::SimpleAction::new("settings", None);
        settings_action.connect_activate(move |_, _| {
            unsafe { c::popover_open_settings_panel(CString::new("region").unwrap().as_ptr()) };
        });

        let action_group = gio::SimpleActionGroup::new();
        action_group.add_action(&layout_action);
        action_group.add_action(&settings_action);

        menu.insert_action_group("popup", Some(&action_group));
    };

    menu.bind_model(Some(&model), Some("popup"));
    menu.popup();
}
