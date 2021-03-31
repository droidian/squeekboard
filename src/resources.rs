/*! Statically linked resources.
 * This could be done using GResource, but that would need additional work.
 */

use std::collections::HashMap;
use ::locale::Translation;

use std::iter::FromIterator;

// TODO: keep a list of what is a language layout,
// and what a convenience layout. "_wide" is not a layout,
// neither is "number"
/// List of builtin layouts
const KEYBOARDS: &[(*const str, *const str)] = &[
    // layouts: us must be left as first, as it is the,
    // fallback layout.
    ("us", include_str!("../data/keyboards/us.yaml")),
    ("us_wide", include_str!("../data/keyboards/us_wide.yaml")),

    // Language layouts: keep alphabetical.
    ("be", include_str!("../data/keyboards/be.yaml")),
    ("be_wide", include_str!("../data/keyboards/be_wide.yaml")),

    ("bg", include_str!("../data/keyboards/bg.yaml")),

    ("br", include_str!("../data/keyboards/br.yaml")),

    ("de", include_str!("../data/keyboards/de.yaml")),
    ("de_wide", include_str!("../data/keyboards/de_wide.yaml")),

    ("cz", include_str!("../data/keyboards/cz.yaml")),
    ("cz_wide", include_str!("../data/keyboards/cz_wide.yaml")),

    ("cz+qwerty", include_str!("../data/keyboards/cz+qwerty.yaml")),
    ("cz+qwerty_wide", include_str!("../data/keyboards/cz+qwerty_wide.yaml")),

    ("dk", include_str!("../data/keyboards/dk.yaml")),

    ("epo", include_str!("../data/keyboards/epo.yaml")),

    ("es", include_str!("../data/keyboards/es.yaml")),
    ("es+cat", include_str!("../data/keyboards/es+cat.yaml")),

    ("fi", include_str!("../data/keyboards/fi.yaml")),

    ("fr", include_str!("../data/keyboards/fr.yaml")),
    ("fr_wide", include_str!("../data/keyboards/fr_wide.yaml")),

    ("gr", include_str!("../data/keyboards/gr.yaml")),

    ("il", include_str!("../data/keyboards/il.yaml")),
    
    ("ir", include_str!("../data/keyboards/ir.yaml")),
    ("ir_wide", include_str!("../data/keyboards/ir_wide.yaml")),

    ("it", include_str!("../data/keyboards/it.yaml")),
    ("it+fur", include_str!("../data/keyboards/it+fur.yaml")),

    ("jp+kana", include_str!("../data/keyboards/jp+kana.yaml")),
    ("jp+kana_wide", include_str!("../data/keyboards/jp+kana_wide.yaml")),

    ("no", include_str!("../data/keyboards/no.yaml")),

    ("pl", include_str!("../data/keyboards/pl.yaml")),
    ("pl_wide", include_str!("../data/keyboards/pl_wide.yaml")),

    ("ru", include_str!("../data/keyboards/ru.yaml")),

    ("se", include_str!("../data/keyboards/se.yaml")),

    ("th", include_str!("../data/keyboards/th.yaml")),
    ("th_wide", include_str!("../data/keyboards/th_wide.yaml")),

    ("ua", include_str!("../data/keyboards/ua.yaml")),

    ("us+colemak", include_str!("../data/keyboards/us+colemak.yaml")),
    ("us+colemak_wide", include_str!("../data/keyboards/us+colemak_wide.yaml")),

    ("us+dvorak", include_str!("../data/keyboards/us+dvorak.yaml")),
    ("us+dvorak_wide", include_str!("../data/keyboards/us+dvorak_wide.yaml")),

    // Others
    ("number", include_str!("../data/keyboards/number.yaml")),

    // layout+overlay
    ("terminal", include_str!("../data/keyboards/terminal.yaml")),
    ("terminal_wide", include_str!("../data/keyboards/terminal_wide.yaml")),
    // Overlays
    ("emoji", include_str!("../data/keyboards/emoji.yaml")),
];

pub fn get_keyboard(needle: &str) -> Option<&'static str> {
    // Need to dereference in unsafe code
    // comparing *const str to &str will compare pointers
    KEYBOARDS.iter()
        .find(|(name, _)| {
            let name: *const str = *name;
            (unsafe { &*name }) == needle
        })
        .map(|(_, value)| {
            let value: *const str = *value;
            unsafe { &*value }
        })
}

const OVERLAY_NAMES: &[*const str] = &[
    "emoji",
    "terminal",
];

pub fn get_overlays() -> Vec<&'static str> {
    OVERLAY_NAMES.iter()
        .map(|name| {
            let name: *const str = *name;
            unsafe { &*name }
        }).collect()
}

/// Translations of the layout identifier strings
const LAYOUT_NAMES: &[(*const str, *const str)] = &[
    ("de-DE", include_str!("../data/langs/de-DE.txt")),
    ("en-US", include_str!("../data/langs/en-US.txt")),
    ("es-ES", include_str!("../data/langs/es-ES.txt")),
    ("fur-IT", include_str!("../data/langs/fur-IT.txt")),
    ("he-IL", include_str!("../data/langs/he_IL.txt")),
    ("ja-JP", include_str!("../data/langs/ja-JP.txt")),
    ("pl-PL", include_str!("../data/langs/pl-PL.txt")),
    ("ru-RU", include_str!("../data/langs/ru-RU.txt")),
];

pub fn get_layout_names(lang: &str)
    -> Option<HashMap<&'static str, Translation<'static>>>
{
    let translations = LAYOUT_NAMES.iter()
        .find(|(name, _data)| {
            let name: *const str = *name;
            (unsafe { &*name }) == lang
        })
        .map(|(_name, data)| {
            let data: *const str = *data;
            unsafe { &*data }
        });
    translations.map(make_mapping)
}

fn parse_line(line: &str) -> Option<(&str, Translation)> {
    let comment = line.trim().starts_with("#");
    if comment {
        None
    } else {
        let mut iter = line.splitn(2, " ");
        let name = iter.next().unwrap();
        // will skip empty and unfinished lines
        iter.next().map(|tr| (name, Translation(tr.trim())))
    }
}

fn make_mapping(data: &str) -> HashMap<&str, Translation> {
    HashMap::from_iter(
        data.split("\n")
            .filter_map(parse_line)
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_overlays_present() {
        for name in get_overlays() {
            assert!(get_keyboard(name).is_some());
        }
    }

    #[test]
    fn mapping_line() {
        assert_eq!(
            Some(("name", Translation("translation"))),
            parse_line("name translation")
        );
    }

    #[test]
    fn mapping_bad() {
        assert_eq!(None, parse_line("bad"));
    }

    #[test]
    fn mapping_empty() {
        assert_eq!(None, parse_line(""));
    }

    #[test]
    fn mapping_comment() {
        assert_eq!(None, parse_line("# comment"));
    }

    #[test]
    fn mapping_comment_offset() {
        assert_eq!(None, parse_line("  # comment"));
    }
}
