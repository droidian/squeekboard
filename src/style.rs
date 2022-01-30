/* 
 * Copyright (C) 2000 Red Hat, Inc.
 * Copyright (C) 2019 Purism, SPC
 *
 * This library is free software; you can redistribute it and/or
 * modify it under the terms of the GNU Lesser General Public
 * License as published by the Free Software Foundation; either
 * version 2 of the License, or (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 * Lesser General Public License for more details.
 *
 * You should have received a copy of the GNU Lesser General Public
 * License along with this library. If not, see <http://www.gnu.org/licenses/>.Free
 */

/*! CSS data loading. */

use std::env;
use ::logging;

use glib::prelude::ObjectExt;
use logging::Warn;

/// Gathers stuff defined in C or called by C
pub mod c {
    use super::*;
    use gio;
    use gtk;
    use gtk_sys;
    
    use gtk::prelude::CssProviderExt;
    use glib::translate::ToGlibPtr;

    /// Loads the layout style based on current theme
    /// without having to worry about string allocation
    #[no_mangle]
    pub extern "C"
    fn squeek_load_style() -> *const gtk_sys::GtkCssProvider {
        unsafe { gtk::set_initialized() };
        
        #[cfg(feature = "glib_v0_14")]
        let theme = gtk::Settings::default();
        #[cfg(not(feature = "glib_v0_14"))]
        let theme = gtk::Settings::get_default();
        
        let theme = theme.map(|settings| get_theme_name(&settings));
        
        let css_name = path_from_theme(theme);

        let resource_name = if gio::resources_get_info(
            &css_name,
            gio::ResourceLookupFlags::NONE
        ).is_ok() {
            css_name
        } else { // use default if this path doesn't exist
            path_from_theme(None)
        };

        let provider = gtk::CssProvider::new();
        provider.load_from_resource(&resource_name);
        provider.to_glib_full()
    }
}

// not Adwaita, but rather fall back to default
const DEFAULT_THEME_NAME: &str = "";

struct GtkTheme {
    name: String,
    variant: Option<String>,
}

/// Gets theme as determined by the toolkit
/// Ported from GTK's gtksettings.c
fn get_theme_name(settings: &gtk::Settings) -> GtkTheme {
    let env_theme = env::var("GTK_THEME")
        .map(|theme| {
            let mut parts = theme.splitn(2, ":");
            GtkTheme {
                // guaranteed at least empty string
                // as the first result from splitting a string
                name: parts.next().unwrap().into(),
                variant: parts.next().map(String::from)
            }
        })
        .map_err(|e| {
            match &e {
                env::VarError::NotPresent => {},
                // maybe TODO: forward this warning?
                e => log_print!(
                    logging::Level::Surprise,
                    "GTK_THEME variable invalid: {}", e,
                ),
            };
            e
        }).ok();

    #[cfg(feature = "glib_v0_14")]
    let prop = |s: &gtk::Settings, name| s.property(name);
    #[cfg(not(feature = "glib_v0_14"))]
    let prop = |s: &gtk::Settings, name| s.get_property(name);

    #[cfg(feature = "glib_v0_14")]
    fn check<T, E: std::fmt::Display>(v: Result<T, E>) -> Option<T> {
        v.or_print(logging::Problem::Surprise, "Key not of expected type")
    }
    #[cfg(not(feature = "glib_v0_14"))]
    fn check<T>(v: Option<T>) -> Option<T> { v }

    match env_theme {
        Some(theme) => theme,
        None => GtkTheme {
            name: {
                prop(settings, "gtk-theme-name")
                    .or_print(logging::Problem::Surprise, "No theme name")
                    .and_then(|value| check(value.get::<String>()))
                    .unwrap_or(DEFAULT_THEME_NAME.into())
            },
            variant: {
                prop(settings, "gtk-application-prefer-dark-theme")
                    .or_print(logging::Problem::Surprise, "No settings key")
                    .and_then(|value| check(value.get::<bool>()))
                    .and_then(|dark_preferred| match dark_preferred {
                        true => Some("dark".into()),
                        false => None,
                    })
            },
        },
    }
}

fn path_from_theme(theme: Option<GtkTheme>) -> String {
    format!(
        "/sm/puri/squeekboard/style{}.css",
        match theme {
            Some(GtkTheme { name, variant: Some(variant) }) => {
                format!("-{}:{}", name, variant)
            },
            Some(GtkTheme { name, variant: None }) => format!("-{}", name),
            None => "".into(),
        }
    )
}
