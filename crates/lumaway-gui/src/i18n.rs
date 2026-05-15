use gettextrs::{
    bind_textdomain_codeset, bindtextdomain, gettext, ngettext, setlocale, textdomain,
    LocaleCategory,
};
use std::path::PathBuf;

const TEXT_DOMAIN: &str = "lumaway-gui";

pub fn init_i18n() {
    let _ = setlocale(LocaleCategory::LcAll, "");
    let locale_dir = locale_dir();
    let _ = bindtextdomain(TEXT_DOMAIN, &locale_dir);
    let _ = bind_textdomain_codeset(TEXT_DOMAIN, "UTF-8");
    let _ = textdomain(TEXT_DOMAIN);
}

#[allow(dead_code)]
pub fn tr(message: &str) -> String {
    gettext(message)
}

#[allow(dead_code)]
pub fn trn(singular: &str, plural: &str, count: u32) -> String {
    ngettext(singular, plural, count)
}

#[allow(dead_code)]
pub fn tr_format(message: &str, values: &[(&str, &str)]) -> String {
    let mut translated = tr(message);
    for (key, value) in values {
        translated = translated.replace(&format!("{{{key}}}"), value);
    }
    translated
}

fn locale_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("LUMAWAY_LOCALEDIR") {
        if !path.is_empty() {
            return PathBuf::from(path);
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".local/share/locale");
    }
    PathBuf::from("/usr/share/locale")
}
