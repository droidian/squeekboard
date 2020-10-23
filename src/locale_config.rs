/*! Locale detection and management.
 * Based on https://github.com/rust-locale/locale_config
 * 
 * Ready for deletion/replacement once Debian starts packaging this,
 * although this version doesn't need lazy_static.
 * 
 * Copyright (c) 2016–2019 Jan Hudec <bulb@ucw.cz>
   Copyright (c) 2016 A.J. Gardner <aaron.j.gardner@gmail.com>
   Copyright (c) 2019, Bastien Orivel <eijebong@bananium.fr>
   Copyright (c) 2019, Igor Gnatenko <i.gnatenko.brain@gmail.com>
   Copyright (c) 2019, Sophie Tauchert <999eagle@999eagle.moe>
 */

use regex::Regex;
use std::borrow::Cow;
use std::env;

/// Errors that may be returned by `locale_config`.
#[derive(Copy,Clone,Debug,PartialEq,Eq)]
pub enum Error {
    /// Provided definition was not well formed.
    ///
    /// This is returned when provided configuration string does not match even the rather loose
    /// definition for language range from [RFC4647] or the composition format used by `Locale`.
    ///
    /// [RFC4647]: https://www.rfc-editor.org/rfc/rfc4647.txt
    NotWellFormed,
    /// Placeholder for adding more errors in future. **Do not match!**.
    __NonExhaustive,
}

impl ::std::fmt::Display for Error {
    fn fmt(&self, out: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        out.write_str(match self {
            &Error::NotWellFormed => "Language tag is not well-formed.",
            // this is exception: here we do want exhaustive match so we don't publish version with
            // missing descriptions by mistake.
            &Error::__NonExhaustive => panic!("Placeholder error must not be instantiated!"),
        })
    }
}


/// Convenience Result alias.
type Result<T> = ::std::result::Result<T, Error>;

/// Iterator over `LanguageRange`s for specific category in a `Locale`
///
/// Returns `LanguageRange`s in the `Locale` that are applicable to provided category. The tags
/// are returned in order of preference, which means the category-specific ones first and then
/// the generic ones.
///
/// The iterator is guaranteed to return at least one value.
pub struct TagsFor<'a, 'c> {
    src: &'a str,
    tags: std::str::Split<'a, &'static str>,
    category: Option<&'c str>,
}

impl<'a, 'c> Iterator for TagsFor<'a, 'c> {
    type Item = LanguageRange<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(cat) = self.category {
            while let Some(s) = self.tags.next() {
                if s.starts_with(cat) && s[cat.len()..].starts_with("=") {
                    return Some(
                        LanguageRange { language: Cow::Borrowed(&s[cat.len()+1..]) });
                }
            }
            self.category = None;
            self.tags = self.src.split(",");
        }
        while let Some(s) = self.tags.next() {
            if s.find('=').is_none() {
                return Some(
                    LanguageRange{ language: Cow::Borrowed(s) });
            }
        }
        return None;
    }
}

/// Language and culture identifier.
///
/// This object holds a [RFC4647] extended language range.
///
/// The internal data may be owned or shared from object with lifetime `'a`. The lifetime can be
/// extended using the `into_static()` method, which internally clones the data as needed.
///
/// # Syntax
///
/// The range is composed of `-`-separated alphanumeric subtags, possibly replaced by `*`s. It
/// might be empty.
///
/// In agreement with [RFC4647], this object only requires that the tag matches:
///
/// ```ebnf
/// language_tag = (alpha{1,8} | "*")
///                ("-" (alphanum{1,8} | "*"))*
/// ```
///
/// The exact interpretation is up to the downstream localization provider, but it expected that
/// it will be matched against a normalized [RFC5646] language tag, which has the structure:
///
/// ```ebnf
/// language_tag    = language
///                   ("-" script)?
///                   ("-" region)?
///                   ("-" variant)*
///                   ("-" extension)*
///                   ("-" private)?
///
/// language        = alpha{2,3} ("-" alpha{3}){0,3}
///
/// script          = aplha{4}
///
/// region          = alpha{2}
///                 | digit{3}
///
/// variant         = alphanum{5,8}
///                 | digit alphanum{3}
///
/// extension       = [0-9a-wyz] ("-" alphanum{2,8})+
///
/// private         = "x" ("-" alphanum{1,8})+
/// ```
///
///  * `language` is an [ISO639] 2-letter or, where not defined, 3-letter code. A code for
///     macro-language might be followed by code of specific dialect.
///  * `script` is an [ISO15924] 4-letter code.
///  * `region` is either an [ISO3166] 2-letter code or, for areas other than countries, [UN M.49]
///    3-digit numeric code.
///  * `variant` is a string indicating variant of the language.
///  * `extension` and `private` define additional options. The private part has same structure as
///    the Unicode [`-u-` extension][u_ext]. Available options are documented for the facets that
///    use them.
///
/// The values obtained by inspecting the system are normalized according to those rules.
///
/// The content will be case-normalized as recommended in [RFC5646] §2.1.1, namely:
///
///  * `language` is written in lowercase,
///  * `script` is written with first capital,
///  * `country` is written in uppercase and
///  * all other subtags are written in lowercase.
///
/// When detecting system configuration, additional options that may be generated under the
/// [`-u-` extension][u_ext] currently are:
///
/// * `cf` — Currency format (`account` for parenthesized negative values, `standard` for minus
///   sign).
/// * `fw` — First day of week (`mon` to `sun`).
/// * `hc` — Hour cycle (`h12` for 1–12, `h23` for 0–23).
/// * `ms` — Measurement system (`metric` or `ussystem`).
/// * `nu` — Numbering system—only decimal systems are currently used.
/// * `va` — Variant when locale is specified in Unix format and the tag after `@` does not
///   correspond to any variant defined in [Language subtag registry].
///
/// And under the `-x-` extension, following options are defined:
///
/// * `df` — Date format:
///
///     * `iso`: Short date should be in ISO format of `yyyy-MM-dd`.
///
///     For example `-df-iso`.
///
/// * `dm` — Decimal separator for monetary:
///
///     Followed by one or more Unicode codepoints in hexadecimal. For example `-dm-002d` means to
///     use comma.
///
/// * `ds` — Decimal separator for numbers:
///
///     Followed by one or more Unicode codepoints in hexadecimal. For example `-ds-002d` means to
///     use comma.
///
/// * `gm` — Group (thousand) separator for monetary:
///
///     Followed by one or more Unicode codepoints in hexadecimal. For example `-dm-00a0` means to
///     use non-breaking space.
///
/// * `gs` — Group (thousand) separator for numbers:
///
///     Followed by one or more Unicode codepoints in hexadecimal. For example `-ds-00a0` means to
///     use non-breaking space.
///
/// * `ls` — List separator:
///
///     Followed by one or more Unicode codepoints in hexadecimal. For example, `-ds-003b` means to
///     use a semicolon.
///
/// [RFC5646]: https://www.rfc-editor.org/rfc/rfc5646.txt
/// [RFC4647]: https://www.rfc-editor.org/rfc/rfc4647.txt
/// [ISO639]: https://en.wikipedia.org/wiki/ISO_639
/// [ISO15924]: https://en.wikipedia.org/wiki/ISO_15924
/// [ISO3166]: https://en.wikipedia.org/wiki/ISO_3166
/// [UN M.49]: https://en.wikipedia.org/wiki/UN_M.49
/// [u_ext]: http://www.unicode.org/reports/tr35/#u_Extension
/// [Language subtag registry]: https://www.iana.org/assignments/language-subtag-registry/language-subtag-registry
#[derive(Clone,Debug,Eq,Hash,PartialEq)]
pub struct LanguageRange<'a> {
    language: Cow<'a, str>
}

impl<'a> LanguageRange<'a> {
    /// Return LanguageRange for the invariant locale.
    ///
    /// Invariant language is identified simply by empty string.
    pub fn invariant() -> LanguageRange<'static> {
        LanguageRange { language: Cow::Borrowed("") }
    }

    /// Create language tag from Unix/Linux/GNU locale tag.
    ///
    /// Unix locale tags have the form
    ///
    /// > *language* [ `_` *region* ] [ `.` *encoding* ] [ `@` *variant* ]
    ///
    /// The *language* and *region* have the same format as RFC5646. *Encoding* is not relevant
    /// here, since Rust always uses Utf-8. That leaves *variant*, which is unfortunately rather
    /// free-form. So this function will translate known variants to corresponding RFC5646 subtags
    /// and represent anything else with Unicode POSIX variant (`-u-va-`) extension.
    ///
    /// Note: This function is public here for benefit of applications that may come across this
    /// kind of tags from other sources than system configuration.
    pub fn from_unix(s: &str) -> Result<LanguageRange<'static>> {
        let unix_tag_regex = Regex::new(r"(?ix) ^
            (?P<language> [[:alpha:]]{2,3} )
            (?: _  (?P<region> [[:alpha:]]{2} | [[:digit:]]{3} ))?
            (?: \. (?P<encoding> [0-9a-zA-Z-]{1,20} ))?
            (?: @  (?P<variant> [[:alnum:]]{1,20} ))?
        $ ").unwrap();
        
        let unix_invariant_regex = Regex::new(r"(?ix) ^
            (?: c | posix )
            (?: \. (?: [0-9a-zA-Z-]{1,20} ))?
        $ ").unwrap();
        
        if let Some(caps) = unix_tag_regex.captures(s) {
            let src_variant = caps.name("variant").map(|m| m.as_str()).unwrap_or("").to_ascii_lowercase();
            let mut res = caps.name("language").map(|m| m.as_str()).unwrap().to_ascii_lowercase();
            let region = caps.name("region").map(|m| m.as_str()).unwrap_or("");
            let mut script = "";
            let mut variant = "";
            let mut uvariant = "";
            match src_variant.as_ref() {
            // Variants seen in the wild in GNU LibC (via http://lh.2xlibre.net/) or in Debian
            // GNU/Linux Stretch system. Treatment of things not found in RFC5646 subtag registry
            // (http://www.iana.org/assignments/language-subtag-registry/language-subtag-registry)
            // or CLDR according to notes at https://wiki.openoffice.org/wiki/LocaleMapping.
            // Dialects:
                // aa_ER@saaho - NOTE: Can't be found under that name in RFC5646 subtag registry,
                // but there is language Saho with code ssy, which is likely that thing.
                "saaho" if res == "aa" => res = String::from("ssy"),
            // Scripts:
                // @arabic
                "arabic" => script = "Arab",
                // @cyrillic
                "cyrl" => script = "Cyrl",
                "cyrillic" => script = "Cyrl",
                // @devanagari
                "devanagari" => script = "Deva",
                // @hebrew
                "hebrew" => script = "Hebr",
                // tt@iqtelif
                // Neither RFC5646 subtag registry nor CLDR knows anything about this, but as best
                // as I can tell it is Tatar name for Latin (default is Cyrillic).
                "iqtelif" => script = "Latn",
                // @Latn
                "latn" => script = "Latn",
                // @latin
                "latin" => script = "Latn",
                // en@shaw
                "shaw" => script = "Shaw",
            // Variants:
                // sr@ijekavianlatin
                "ijekavianlatin" => {
                    script = "Latn";
                    variant = "ijekavsk";
                },
                // sr@ije
                "ije" => variant = "ijekavsk",
                // sr@ijekavian
                "ijekavian" => variant = "ijekavsk",
                // ca@valencia
                "valencia" => variant = "valencia",
            // Currencies:
                // @euro - NOTE: We follow suite of Java and Openoffice and ignore it, because it
                // is default for all locales where it sometimes appears now, and because we use
                // explicit currency in monetary formatting anyway.
                "euro" => {},
            // Collation:
                // gez@abegede - NOTE: This is collation, but CLDR does not have any code for it,
                // so we for the moment leave it fall through as -u-va- instead of -u-co-.
            // Anything else:
                // en@boldquot, en@quot, en@piglatin - just randomish stuff
                // @cjknarrow - beware, it's gonna end up as -u-va-cjknarro due to lenght limit
                s if s.len() <= 8 => uvariant = &*s,
                s => uvariant = &s[0..8], // the subtags are limited to 8 chars, but some are longer
            };
            if script != "" {
                res.push('-');
                res.push_str(script);
            }
            if region != "" {
                res.push('-');
                res.push_str(&*region.to_ascii_uppercase());
            }
            if variant != "" {
                res.push('-');
                res.push_str(variant);
            }
            if uvariant != "" {
                res.push_str("-u-va-");
                res.push_str(uvariant);
            }
            return Ok(LanguageRange {
                language: Cow::Owned(res)
            });
        } else if unix_invariant_regex.is_match(s) {
            return Ok(LanguageRange::invariant())
        } else {
            return Err(Error::NotWellFormed);
        }
    }
}

impl<'a> AsRef<str> for LanguageRange<'a> {
    fn as_ref(&self) -> &str {
        self.language.as_ref()
    }
}

/// Locale configuration.
///
/// Users may accept several languages in some order of preference and may want to use rules from
/// different culture for some particular aspect of the program behaviour, and operating systems
/// allow them to specify this (to various extent).
///
/// The `Locale` objects represent the user configuration. They contain:
///
///  - The primary `LanguageRange`.
///  - Optional category-specific overrides.
///  - Optional fallbacks in case data (usually translations) for the primary language are not
///    available.
///
/// The set of categories is open-ended. The `locale` crate uses five well-known categories
/// `messages`, `numeric`, `time`, `collate` and `monetary`, but some systems define additional
/// ones (GNU Linux has additionally `paper`, `name`, `address`, `telephone` and `measurement`) and
/// these are provided in the user default `Locale` and other libraries can use them.
///
/// `Locale` is represented by a `,`-separated sequence of tags in `LanguageRange` syntax, where
/// all except the first one may be preceded by category name and `=` sign.
///
/// The first tag indicates the default locale, the tags prefixed by category names indicate
/// _overrides_ for those categories and the remaining tags indicate fallbacks.
///
/// Note that a syntactically valid value of HTTP `Accept-Language` header is a valid `Locale`. Not
/// the other way around though due to the presence of category selectors.
// TODO: Interning
#[derive(Clone,Debug,Eq,Hash,PartialEq)]
pub struct Locale {
    // TODO: Intern the string for performance reasons
    // XXX: Store pre-split to LanguageTags?
    inner: String,
}

impl Locale {
    /// Construct invariant locale.
    ///
    /// Invariant locale is represented simply with empty string.
    pub fn invariant() -> Locale {
        Locale::from(LanguageRange::invariant())
    }

    /// Append fallback language tag.
    ///
    /// Adds fallback to the end of the list.
    pub fn add(&mut self, tag: &LanguageRange) {
        for i in self.inner.split(',') {
            if i == tag.as_ref() {
                return; // don't add duplicates
            }
        }
        self.inner.push_str(",");
        self.inner.push_str(tag.as_ref());
    }

    /// Append category override.
    ///
    /// Appending new override for a category that already has one will not replace the existing
    /// override. This might change in future.
    pub fn add_category(&mut self, category: &str, tag: &LanguageRange) {
        if self.inner.split(',').next().unwrap() == tag.as_ref() {
            return; // don't add useless override equal to the primary tag
        }
        for i in self.inner.split(',') {
            if i.starts_with(category) &&
                    i[category.len()..].starts_with("=") &&
                    &i[category.len() + 1..] == tag.as_ref() {
                return; // don't add duplicates
            }
        }
        self.inner.push_str(",");
        self.inner.push_str(category);
        self.inner.push_str("=");
        self.inner.push_str(tag.as_ref());
    }

    /// Iterate over `LanguageRange`s in this `Locale` applicable to given category.
    ///
    /// Returns `LanguageRange`s in the `Locale` that are applicable to provided category. The tags
    /// are returned in order of preference, which means the category-specific ones first and then
    /// the generic ones.
    ///
    /// The iterator is guaranteed to return at least one value.
    pub fn tags_for<'a, 'c>(&'a self, category: &'c str) -> TagsFor<'a, 'c> {
        let mut tags = self.inner.split(",");
        while let Some(s) = tags.clone().next() {
            if s.starts_with(category) && s[category.len()..].starts_with("=") {
                return TagsFor {
                    src: self.inner.as_ref(),
                    tags: tags,
                    category: Some(category),
                };
            }
            tags.next();
        }
        return TagsFor {
            src: self.inner.as_ref(),
            tags: self.inner.split(","),
            category: None,
        };
    }
}

/// Locale is specified by a string tag. This is the way to access it.
// FIXME: Do we want to provide the full string representation? We would have it as single string
// then.
impl AsRef<str> for Locale {
    fn as_ref(&self) -> &str {
        self.inner.as_ref()
    }
}

impl<'a> From<LanguageRange<'a>> for Locale {
    fn from(t: LanguageRange<'a>) -> Locale {
        Locale {
            inner: t.language.into_owned(),
        }
    }
}

fn tag(s: &str) -> Result<LanguageRange> {
    LanguageRange::from_unix(s)
}

// TODO: Read /etc/locale.alias
fn tag_inv(s: &str) -> LanguageRange {
    tag(s).unwrap_or(LanguageRange::invariant())
}

pub fn system_locale() -> Option<Locale> {
    // LC_ALL overrides everything
    if let Ok(all) = env::var("LC_ALL") {
        if let Ok(t) = tag(all.as_ref()) {
            return Some(Locale::from(t));
        }
    }
    // LANG is default
    let mut loc =
        if let Ok(lang) = env::var("LANG") {
            Locale::from(tag_inv(lang.as_ref()))
        } else {
            Locale::invariant()
        };
    // category overrides
    for &(cat, var) in [
        ("ctype",       "LC_CTYPE"),
        ("numeric",     "LC_NUMERIC"),
        ("time",        "LC_TIME"),
        ("collate",     "LC_COLLATE"),
        ("monetary",    "LC_MONETARY"),
        ("messages",    "LC_MESSAGES"),
        ("paper",       "LC_PAPER"),
        ("name",        "LC_NAME"),
        ("address",     "LC_ADDRESS"),
        ("telephone",   "LC_TELEPHONE"),
        ("measurement", "LC_MEASUREMENT"),
    ].iter() {
        if let Ok(val) = env::var(var) {
            if let Ok(tag) = tag(val.as_ref())
            {
                loc.add_category(cat, &tag);
            }
        }
    }
    // LANGUAGE defines fallbacks
    if let Ok(langs) = env::var("LANGUAGE") {
        for i in langs.split(':') {
            if i != "" {
                if let Ok(tag) = tag(i) {
                    loc.add(&tag);
                }
            }
        }
    }
    if loc.as_ref() != "" {
        return Some(loc);
    } else {
        return None;
    }
}

#[cfg(test)]
mod test {
    use super::LanguageRange;

    #[test]
    fn unix_tags() {
        assert_eq!("cs-CZ", LanguageRange::from_unix("cs_CZ.UTF-8").unwrap().as_ref());
        assert_eq!("sr-RS-ijekavsk", LanguageRange::from_unix("sr_RS@ijekavian").unwrap().as_ref());
        assert_eq!("sr-Latn-ijekavsk", LanguageRange::from_unix("sr.UTF-8@ijekavianlatin").unwrap().as_ref());
        assert_eq!("en-Arab", LanguageRange::from_unix("en@arabic").unwrap().as_ref());
        assert_eq!("en-Arab", LanguageRange::from_unix("en.UTF-8@arabic").unwrap().as_ref());
        assert_eq!("de-DE", LanguageRange::from_unix("DE_de.UTF-8@euro").unwrap().as_ref());
        assert_eq!("ssy-ER", LanguageRange::from_unix("aa_ER@saaho").unwrap().as_ref());
        assert!(LanguageRange::from_unix("foo_BAR").is_err());
        assert!(LanguageRange::from_unix("en@arabic.UTF-8").is_err());
        assert_eq!("", LanguageRange::from_unix("C").unwrap().as_ref());
        assert_eq!("", LanguageRange::from_unix("C.UTF-8").unwrap().as_ref());
        assert_eq!("", LanguageRange::from_unix("C.ISO-8859-1").unwrap().as_ref());
        assert_eq!("", LanguageRange::from_unix("POSIX").unwrap().as_ref());
    }
}
