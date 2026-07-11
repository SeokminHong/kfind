use std::env;
use std::ffi::{OsStr, OsString};

const LOCALE_VARIABLES: [&str; 3] = ["LC_ALL", "LC_MESSAGES", "LANG"];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Language {
    #[default]
    English,
    Korean,
}

impl Language {
    #[must_use]
    pub fn from_env() -> Self {
        resolve_language(|name| env::var_os(name))
    }

    pub(crate) const fn select<'a>(self, english: &'a str, korean: &'a str) -> &'a str {
        match self {
            Self::English => english,
            Self::Korean => korean,
        }
    }
}

fn resolve_language(mut read: impl FnMut(&str) -> Option<OsString>) -> Language {
    LOCALE_VARIABLES
        .iter()
        .find_map(|name| read(name).filter(|value| !value.is_empty()))
        .map_or(Language::English, |locale| language_from_locale(&locale))
}

fn language_from_locale(locale: &OsStr) -> Language {
    let Some(locale) = locale.to_str() else {
        return Language::English;
    };
    let language = locale
        .split(['_', '-', '.', '@'])
        .next()
        .unwrap_or_default();
    if language.eq_ignore_ascii_case("ko") {
        Language::Korean
    } else {
        Language::English
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(lc_all: Option<&str>, lc_messages: Option<&str>, lang: Option<&str>) -> Language {
        resolve_language(|name| {
            match name {
                "LC_ALL" => lc_all,
                "LC_MESSAGES" => lc_messages,
                "LANG" => lang,
                _ => None,
            }
            .map(OsString::from)
        })
    }

    #[test]
    fn locale_variables_follow_posix_precedence() {
        assert_eq!(
            resolve(Some("ko_KR.UTF-8"), Some("C"), Some("en")),
            Language::Korean
        );
        assert_eq!(
            resolve(Some("C"), Some("ko"), Some("ko")),
            Language::English
        );
        assert_eq!(resolve(Some(""), Some("ko"), Some("C")), Language::Korean);
        assert_eq!(resolve(None, Some("C"), Some("ko")), Language::English);
        assert_eq!(
            resolve(Some("fr_FR"), Some("ko"), Some("ko")),
            Language::English
        );
    }

    #[test]
    fn korean_locale_variants_are_recognized() {
        for locale in [
            "ko",
            "KO",
            "ko_KR",
            "ko-KR",
            "ko_KR.UTF-8",
            "ko_KR.UTF-8@modifier",
        ] {
            assert_eq!(resolve(None, None, Some(locale)), Language::Korean);
        }
    }

    #[test]
    fn unset_c_and_unsupported_locales_use_english() {
        for locale in [
            "",
            "C",
            "POSIX",
            "C.UTF-8",
            "en_US.UTF-8",
            "fr_FR",
            "kok_IN",
        ] {
            assert_eq!(resolve(None, None, Some(locale)), Language::English);
        }
        assert_eq!(resolve(None, None, None), Language::English);
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_locale_uses_english() {
        use std::os::unix::ffi::OsStringExt;

        let locale = OsString::from_vec(vec![b'k', b'o', 0xff]);
        assert_eq!(
            resolve_language(|name| (name == "LANG").then(|| locale.clone())),
            Language::English
        );
    }
}
