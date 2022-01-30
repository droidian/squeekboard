/*! Statically linked resources.
 * This could be done using GResource, but that would need additional work.
 */

// TODO: keep a list of what is a language layout,
// and what a convenience layout. "_wide" is not a layout,
// neither is "number"
/// List of builtin layouts
static KEYBOARDS: &[(&'static str, &'static str)] = &[
    // layouts: us must be left as first, as it is the,
    // fallback layout.
    ("us", include_str!("../data/keyboards/us.yaml")),
    ("us_wide", include_str!("../data/keyboards/us_wide.yaml")),

    // Language layouts: keep alphabetical.
    ("am", include_str!("../data/keyboards/am.yaml")),
    ("am+phonetic", include_str!("../data/keyboards/am+phonetic.yaml")),

    ("ara", include_str!("../data/keyboards/ara.yaml")),
    ("ara_wide", include_str!("../data/keyboards/ara_wide.yaml")),

    ("be", include_str!("../data/keyboards/be.yaml")),
    ("be_wide", include_str!("../data/keyboards/be_wide.yaml")),

    ("bg", include_str!("../data/keyboards/bg.yaml")),
    ("bg+phonetic", include_str!("../data/keyboards/bg+phonetic.yaml")),

    ("br", include_str!("../data/keyboards/br.yaml")),
    
    ("ch+fr", include_str!("../data/keyboards/ch+fr.yaml")),
    ("ch+de", include_str!("../data/keyboards/ch+de.yaml")),
    ("ch", include_str!("../data/keyboards/ch.yaml")),
    ("ch_wide", include_str!("../data/keyboards/ch_wide.yaml")),

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

    ("ro", include_str!("../data/keyboards/ro.yaml")),
    ("ro_wide", include_str!("../data/keyboards/ro_wide.yaml")),

    ("ru", include_str!("../data/keyboards/ru.yaml")),

    ("se", include_str!("../data/keyboards/se.yaml")),

    ("th", include_str!("../data/keyboards/th.yaml")),
    ("th_wide", include_str!("../data/keyboards/th_wide.yaml")),

    ("ua", include_str!("../data/keyboards/ua.yaml")),

    ("us+colemak", include_str!("../data/keyboards/us+colemak.yaml")),
    ("us+colemak_wide", include_str!("../data/keyboards/us+colemak_wide.yaml")),

    ("us+dvorak", include_str!("../data/keyboards/us+dvorak.yaml")),
    ("us+dvorak_wide", include_str!("../data/keyboards/us+dvorak_wide.yaml")),

    // Email
    ("email/us", include_str!("../data/keyboards/email/us.yaml")),

    // URL
    ("url/us", include_str!("../data/keyboards/url/us.yaml")),

    // Others
    ("number/us", include_str!("../data/keyboards/number/us.yaml")),
    ("pin/us", include_str!("../data/keyboards/pin/us.yaml")),

    // Terminal
    ("terminal/fr", include_str!("../data/keyboards/terminal/fr.yaml")),
    ("terminal/fr_wide", include_str!("../data/keyboards/terminal/fr_wide.yaml")),

    ("terminal/us", include_str!("../data/keyboards/terminal/us.yaml")),
    ("terminal/us_wide",   include_str!("../data/keyboards/terminal/us_wide.yaml")),

    // Overlays
    ("emoji/us", include_str!("../data/keyboards/emoji/us.yaml")),
];

pub fn get_keyboard(needle: &str) -> Option<&'static str> {
    KEYBOARDS.iter().find(|(name, _)| *name == needle).map(|(_, layout)| *layout)
}

static OVERLAY_NAMES: &[&'static str] = &[
    "emoji",
    "terminal",
];

pub fn get_overlays() -> Vec<&'static str> {
    OVERLAY_NAMES.to_vec()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_overlays_present() {
        for name in get_overlays() {
            assert!(get_keyboard(&format!("{}/us", name)).is_some());
        }
    }
}
