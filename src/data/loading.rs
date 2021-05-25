/* Copyright (C) 2020-2021 Purism SPC
 * SPDX-License-Identifier: GPL-3.0+
 */

/*! Loading layout files */

use std::env;
use std::fmt;
use std::path::PathBuf;
use std::convert::TryFrom;

use super::{ Error, LoadError };
use super::parsing;

use ::layout::ArrangementKind;
use ::logging;
use ::util::c::as_str;
use ::xdg;
use ::imservice::ContentPurpose;

// traits, derives
use ::logging::Warn;


/// Gathers stuff defined in C or called by C
pub mod c {
    use super::*;
    use std::os::raw::c_char;

    #[no_mangle]
    pub extern "C"
    fn squeek_load_layout(
        name: *const c_char,    // name of the keyboard
        type_: u32,             // type like Wide
        variant: u32,          // purpose variant like numeric, terminal...
        overlay: *const c_char, // the overlay (looking for "terminal")
    ) -> *mut ::layout::Layout {
        let type_ = match type_ {
            0 => ArrangementKind::Base,
            1 => ArrangementKind::Wide,
            _ => panic!("Bad enum value"),
        };
        
        let name = as_str(&name)
            .expect("Bad layout name")
            .expect("Empty layout name");

        let variant = ContentPurpose::try_from(variant)
                    .or_print(
                        logging::Problem::Warning,
                        "Received invalid purpose value",
                    )
                    .unwrap_or(ContentPurpose::Normal);

        let overlay_str = as_str(&overlay)
                .expect("Bad overlay name")
                .expect("Empty overlay name");
        let overlay_str = match overlay_str {
            "" => None,
            other => Some(other),
        };

        let (kind, layout) = load_layout_data_with_fallback(&name, type_, variant, overlay_str);
        let layout = ::layout::Layout::new(layout, kind);
        Box::into_raw(Box::new(layout))
    }
}

const FALLBACK_LAYOUT_NAME: &str = "us";


#[derive(Debug, Clone, PartialEq)]
enum DataSource {
    File(PathBuf),
    Resource(String),
}

impl fmt::Display for DataSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataSource::File(path) => write!(f, "Path: {:?}", path.display()),
            DataSource::Resource(name) => write!(f, "Resource: {}", name),
        }
    }
}

/* All functions in this family carry around ArrangementKind,
 * because it's not guaranteed to be preserved,
 * and the resulting layout needs to know which version was loaded.
 * See `squeek_layout_get_kind`.
 * Possible TODO: since this is used only in styling,
 * and makes the below code nastier than needed, maybe it should go.
 */

/// Returns ordered names treating `name` as the base name,
/// ignoring any `+` inside.
fn _get_arrangement_names(name: &str, arrangement: ArrangementKind)
    -> Vec<(ArrangementKind, String)>
{
    let name_with_arrangement = match arrangement {    
        ArrangementKind::Base => name.into(),
        ArrangementKind::Wide => format!("{}_wide", name),
    };
    
    let mut ret = Vec::new();
    if name_with_arrangement != name {
        ret.push((arrangement, name_with_arrangement));
    }
    ret.push((ArrangementKind::Base, name.into()));
    ret
}

/// Returns names accounting for any `+` in the `name`,
/// including the fallback to the default layout.
fn get_preferred_names(name: &str, kind: ArrangementKind)
    -> Vec<(ArrangementKind, String)>
{
    let mut ret = _get_arrangement_names(name, kind);
    
    let base_name_preferences = {
        let mut parts = name.splitn(2, '+');
        match parts.next() {
            Some(base) => {
                // The name is already equal to base, so nothing to add
                if base == name {
                    vec![]
                } else {
                    _get_arrangement_names(base, kind)
                }
            },
            // The layout's base name starts with a "+". Weird but OK.
            None => {
                log_print!(logging::Level::Surprise, "Base layout name is empty: {}", name);
                vec![]
            }
        }
    };
    
    ret.extend(base_name_preferences.into_iter());
    let fallback_names = _get_arrangement_names(FALLBACK_LAYOUT_NAME, kind);
    ret.extend(fallback_names.into_iter());
    ret
}

/// Includes the subdirectory before the forward slash.
type LayoutPath = String;

// This is only used inside iter_fallbacks_with_meta.
// Placed at the top scope
// because `use LayoutPurpose::*;`
// complains about "not in scope" otherwise.
// This seems to be a Rust 2015 edition problem.
/// Helper for determining where to look up the layout.
enum LayoutPurpose<'a> {
    Default,
    Special(&'a str),
}

/// Returns the directory string
/// where the layout should be looked up, including the slash.
fn get_directory_string(
    content_purpose: ContentPurpose,
    overlay: Option<&str>) -> String
{
    use self::LayoutPurpose::*;

    let layout_purpose = match overlay {
        None => match content_purpose {
            ContentPurpose::Number => Special("number"),
            ContentPurpose::Digits => Special("number"),
            ContentPurpose::Phone => Special("number"),
            ContentPurpose::Terminal => Special("terminal"),
            _ => Default,
        },
        Some(overlay) => Special(overlay),
    };

    // For intuitiveness,
    // default purpose layouts are stored in the root directory,
    // as they correspond to typical text
    // and are seen the most often.
    match layout_purpose {
        Default => "".into(),
        Special(purpose) => format!("{}/", purpose),
    }
}

/// Returns an iterator over all fallback paths.
fn to_layout_paths(
    name_fallbacks: Vec<(ArrangementKind, String)>,
    content_purpose: ContentPurpose,
    overlay: Option<&str>,
) -> impl Iterator<Item=(ArrangementKind, LayoutPath)> {
    let prepend_directory = get_directory_string(content_purpose, overlay);

    name_fallbacks.into_iter()
        .map(move |(arrangement, name)|
            (arrangement, format!("{}{}", prepend_directory, name))
        )
}

type LayoutSource = (ArrangementKind, DataSource);

fn to_layout_sources(
    layout_paths: impl Iterator<Item=(ArrangementKind, LayoutPath)>,
    filesystem_path: Option<PathBuf>,
) -> impl Iterator<Item=LayoutSource> {
    layout_paths.flat_map(move |(arrangement, layout_path)| {
        let mut sources = Vec::new();
        if let Some(path) = &filesystem_path {
            sources.push((
                arrangement,
                DataSource::File(
                    path.join(&layout_path)
                        .with_extension("yaml")
                )
            ));
        };
        sources.push((arrangement, DataSource::Resource(layout_path.clone())));
        sources.into_iter()
    })
}

/// Returns possible sources, with first as the most preferred one.
/// Trying order: native lang of the right kind, native base,
/// fallback lang of the right kind, fallback base
fn iter_layout_sources(
    name: &str,
    arrangement: ArrangementKind,
    purpose: ContentPurpose,
    ui_overlay: Option<&str>,
    layout_storage: Option<PathBuf>,
) -> impl Iterator<Item=LayoutSource> {
    let names = get_preferred_names(name, arrangement);
    let paths = to_layout_paths(names, purpose, ui_overlay);
    to_layout_sources(paths, layout_storage)
}

fn load_layout_data(source: DataSource)
    -> Result<::layout::LayoutData, LoadError>
{
    let handler = logging::Print {};
    match source {
        DataSource::File(path) => {
            parsing::Layout::from_file(path.clone())
                .map_err(LoadError::BadData)
                .and_then(|layout|
                    layout.build(handler).0.map_err(LoadError::BadKeyMap)
                )
        },
        DataSource::Resource(name) => {
            parsing::Layout::from_resource(&name)
                .and_then(|layout|
                    layout.build(handler).0.map_err(LoadError::BadKeyMap)
                )
        },
    }
}

fn load_layout_data_with_fallback(
    name: &str,
    kind: ArrangementKind,
    purpose: ContentPurpose,
    overlay: Option<&str>,
) -> (ArrangementKind, ::layout::LayoutData) {

    // Build the path to the right keyboard layout subdirectory
    let path = env::var_os("SQUEEKBOARD_KEYBOARDSDIR")
        .map(PathBuf::from)
        .or_else(|| xdg::data_path("squeekboard/keyboards"));

    for (kind, source) in iter_layout_sources(&name, kind, purpose, overlay, path) {
        let layout = load_layout_data(source.clone());
        match layout {
            Err(e) => match (e, source) {
                (
                    LoadError::BadData(Error::Missing(e)),
                    DataSource::File(file)
                ) => log_print!(
                    logging::Level::Debug,
                    "Tried file {:?}, but it's missing: {}",
                    file, e
                ),
                (e, source) => log_print!(
                    logging::Level::Warning,
                    "Failed to load layout from {}: {}, skipping",
                    source, e
                ),
            },
            Ok(layout) => {
                log_print!(logging::Level::Info, "Loaded layout {}", source);
                return (kind, layout);
            }
        }
    }

    panic!("No useful layout found!");
}


#[cfg(test)]
mod tests {
    use super::*;

    use ::logging::ProblemPanic;

    #[test]
    fn parsing_fallback() {
        assert!(parsing::Layout::from_resource(FALLBACK_LAYOUT_NAME)
            .map(|layout| layout.build(ProblemPanic).0.unwrap())
            .is_ok()
        );
    }
    
    /// First fallback should be to builtin, not to FALLBACK_LAYOUT_NAME
    #[test]
    fn test_fallback_basic_builtin() {
        let sources = iter_layout_sources("nb", ArrangementKind::Base, ContentPurpose::Normal, None, None);
        
        assert_eq!(
            sources.collect::<Vec<_>>(),
            vec!(
                (ArrangementKind::Base, DataSource::Resource("nb".into())),
                (
                    ArrangementKind::Base,
                    DataSource::Resource(FALLBACK_LAYOUT_NAME.into())
                ),
            )
        );
    }
    
    /// Prefer loading from file system before builtin.
    #[test]
    fn test_preferences_order_path() {
        let sources = iter_layout_sources("nb", ArrangementKind::Base, ContentPurpose::Normal, None, Some(".".into()));
        
        assert_eq!(
            sources.collect::<Vec<_>>(),
            vec!(
                (ArrangementKind::Base, DataSource::File("./nb.yaml".into())),
                (ArrangementKind::Base, DataSource::Resource("nb".into())),
                (
                    ArrangementKind::Base,
                    DataSource::File("./us.yaml".into())
                ),
                (
                    ArrangementKind::Base,
                    DataSource::Resource("us".into())
                ),
            )
        );
    }

    /// If layout contains a "+", it should reach for what's in front of it too.
    #[test]
    fn test_preferences_order_base() {
        let sources = iter_layout_sources("nb+aliens", ArrangementKind::Base, ContentPurpose::Normal, None, None);

        assert_eq!(
            sources.collect::<Vec<_>>(),
            vec!(
                (ArrangementKind::Base, DataSource::Resource("nb+aliens".into())),
                (ArrangementKind::Base, DataSource::Resource("nb".into())),
                (
                    ArrangementKind::Base,
                    DataSource::Resource(FALLBACK_LAYOUT_NAME.into())
                ),
            )
        );
    }

    #[test]
    fn test_preferences_order_arrangement() {
        let sources = iter_layout_sources("nb", ArrangementKind::Wide, ContentPurpose::Normal, None, None);

        assert_eq!(
            sources.collect::<Vec<_>>(),
            vec!(
                (ArrangementKind::Wide, DataSource::Resource("nb_wide".into())),
                (ArrangementKind::Base, DataSource::Resource("nb".into())),
                (
                    ArrangementKind::Wide,
                    DataSource::Resource("us_wide".into())
                ),
                (
                    ArrangementKind::Base,
                    DataSource::Resource("us".into())
                ),
            )
        );
    }

    #[test]
    fn test_preferences_order_overlay() {
        let sources = iter_layout_sources("nb", ArrangementKind::Base, ContentPurpose::Normal, Some("terminal"), None);

        assert_eq!(
            sources.collect::<Vec<_>>(),
            vec!(
                (ArrangementKind::Base, DataSource::Resource("terminal/nb".into())),
                (
                    ArrangementKind::Base,
                    DataSource::Resource("terminal/us".into())
                ),
            )
        );
    }

    #[test]
    fn test_preferences_order_hint() {
        let sources = iter_layout_sources("nb", ArrangementKind::Base, ContentPurpose::Terminal, None, None);

        assert_eq!(
            sources.collect::<Vec<_>>(),
            vec!(
                (ArrangementKind::Base, DataSource::Resource("terminal/nb".into())),
                (
                    ArrangementKind::Base,
                    DataSource::Resource("terminal/us".into())
                ),
            )
        );
    }
}
