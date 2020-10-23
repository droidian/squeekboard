/*! Statically linked resources.
 * This could be done using GResource, but that would need additional work.
 */

use std::collections::HashMap;
use ::locale::Translation;

use std::iter::FromIterator;

// TODO: keep a list of what is a language layout,
// and what a convenience layout. "_wide" is not a layout,
// neither is "number"
const KEYBOARDS: &[(*const str, *const str)] = &[
    // layouts: us must be left as first, as it is the,
    // fallback layout. The others should be alphabetical.
    ("us", include_str!("../data/keyboards/us.yaml")),
    ("us_wide", include_str!("../data/keyboards/us_wide.yaml")),
    ("br", include_str!("../data/keyboards/br.yaml")),
    ("de", include_str!("../data/keyboards/de.yaml")),
    ("be", include_str!("../data/keyboards/be.yaml")),
    ("be_wide", include_str!("../data/keyboards/be_wide.yaml")),
    ("de_wide", include_str!("../data/keyboards/de_wide.yaml")),
    ("dk", include_str!("../data/keyboards/dk.yaml")),
    ("es", include_str!("../data/keyboards/es.yaml")),
    ("fi", include_str!("../data/keyboards/fi.yaml")),
    ("fr", include_str!("../data/keyboards/fr.yaml")),
    ("fr_wide", include_str!("../data/keyboards/fr_wide.yaml")),
    ("it+fur", include_str!("../data/keyboards/it+fur.yaml")),
    ("gr", include_str!("../data/keyboards/gr.yaml")),
    ("it", include_str!("../data/keyboards/it.yaml")),
    ("jp+kana", include_str!("../data/keyboards/jp+kana.yaml")),
    ("jp+kana_wide", include_str!("../data/keyboards/jp+kana_wide.yaml")),
    ("no", include_str!("../data/keyboards/no.yaml")),
    ("number", include_str!("../data/keyboards/number.yaml")),
    ("pl", include_str!("../data/keyboards/pl.yaml")),
    ("pl_wide", include_str!("../data/keyboards/pl_wide.yaml")),
    ("ru", include_str!("../data/keyboards/ru.yaml")),
    ("se", include_str!("../data/keyboards/se.yaml")),
    ("ua", include_str!("../data/keyboards/ua.yaml")),
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
