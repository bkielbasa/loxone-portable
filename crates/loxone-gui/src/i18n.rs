use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use loxone_core::miniserver::ConnectionProfile;
use serde::Deserialize;

/// Open-ended language identifier backed by an interned lowercase locale code (e.g. `"pl"`, `"en"`, `"de"`, `"cs"`, `"es"`).
///
/// New languages can be added by dropping `<code.json>` files into `./locales/` or
/// `~/.config/loxone-portable/locales/`, or at runtime via [`register_locale_json`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Language(pub &'static str);

#[allow(non_upper_case_globals)]
impl Language {
    pub const Pl: Self = Self("pl");
    pub const En: Self = Self("en");
    pub const De: Self = Self("de");

    /// Normalizes a language code or name (e.g. `"PL"`, `"pl-PL"`, `"en"`, `"de"`, `"cs"`) into a [`Language`].
    pub fn from_code(code: &str) -> Self {
        let trimmed = code.trim();
        if trimmed.is_empty() {
            return Self::Pl;
        }
        let primary = trimmed
            .split(['-', '_', ' '])
            .next()
            .unwrap_or(trimmed)
            .to_ascii_lowercase();
        let normalized = match primary.as_str() {
            "pl" | "pol" | "polski" | "polish" => "pl",
            "en" | "eng" | "english" => "en",
            "de" | "deu" | "ger" | "deutsch" | "german" => "de",
            other => other,
        };
        let reg = registry().read().unwrap();
        if let Some(pack) = reg.packs.get(normalized) {
            return Language(pack.id);
        }
        drop(reg);
        Language(intern_static_str(normalized))
    }

    /// Uppercase badge code for toolbar/UI display (e.g. `"PL"`, `"EN"`, `"DE"`).
    pub fn code(self) -> &'static str {
        let reg = registry().read().unwrap();
        reg.packs
            .get(self.0)
            .map(|p| p.code)
            .unwrap_or(self.0)
    }

    /// Native language name from the locale pack (e.g. `"Polski"`, `"English"`, `"Deutsch"`).
    pub fn native_name(self) -> &'static str {
        let reg = registry().read().unwrap();
        reg.packs
            .get(self.0)
            .map(|p| p.native_name)
            .unwrap_or(self.0)
    }

    pub fn display_name(self) -> &'static str {
        self.native_name()
    }

    /// Cycles to the next registered language in [`available_languages`].
    pub fn next(self) -> Self {
        let langs = available_languages();
        if langs.is_empty() {
            return Self::Pl;
        }
        if let Some(pos) = langs.iter().position(|&l| l == self) {
            langs[(pos + 1) % langs.len()]
        } else {
            langs[0]
        }
    }
}

impl Default for Language {
    fn default() -> Self {
        Self::Pl
    }
}

#[derive(Debug, Deserialize)]
struct RawLocaleFile {
    code: String,
    native_name: String,
    #[serde(default)]
    strings: HashMap<String, String>,
    #[serde(default)]
    block_names: HashMap<String, String>,
    #[serde(default)]
    categories: HashMap<String, String>,
}

struct LocalePack {
    id: &'static str,
    code: &'static str,
    native_name: &'static str,
    strings: HashMap<&'static str, &'static str>,
    block_names: HashMap<&'static str, &'static str>,
    categories: HashMap<&'static str, &'static str>,
}

struct I18nRegistry {
    active: Language,
    order: Vec<Language>,
    packs: HashMap<&'static str, LocalePack>,
    interned: HashMap<String, &'static str>,
}

impl I18nRegistry {
    fn new() -> Self {
        let mut reg = Self {
            active: Language::Pl,
            order: Vec::new(),
            packs: HashMap::new(),
            interned: HashMap::new(),
        };

        // Built-in embedded locale packs
        for json_str in [
            include_str!("../locales/pl.json"),
            include_str!("../locales/en.json"),
            include_str!("../locales/de.json"),
        ] {
            let _ = reg.load_json_str(json_str);
        }

        // External locale directories for drop-in extensibility without recompiling
        for dir in candidate_locale_dirs() {
            reg.load_dir(&dir);
        }

        reg
    }

    fn intern(&mut self, s: &str) -> &'static str {
        if let Some(&existing) = self.interned.get(s) {
            return existing;
        }
        let leaked: &'static str = Box::leak(s.to_string().into_boxed_str());
        self.interned.insert(s.to_string(), leaked);
        leaked
    }

    fn load_json_str(&mut self, json_str: &str) -> Result<Language, String> {
        let raw: RawLocaleFile =
            serde_json::from_str(json_str).map_err(|e| format!("Invalid locale JSON: {}", e))?;
        let norm_id = raw.code.trim().to_ascii_lowercase();
        if norm_id.is_empty() {
            return Err("Locale JSON 'code' cannot be empty".to_string());
        }
        let id = self.intern(&norm_id);
        let code = self.intern(&raw.code.trim().to_ascii_uppercase());
        let native_name = self.intern(raw.native_name.trim());

        let mut strings = self
            .packs
            .remove(id)
            .map(|p| p.strings)
            .unwrap_or_default();
        let mut block_names = HashMap::new();
        let mut categories = HashMap::new();

        for (k, v) in raw.strings {
            let ik = self.intern(&k);
            let iv = self.intern(&v);
            strings.insert(ik, iv);
        }
        for (k, v) in raw.block_names {
            let ik = self.intern(&k);
            let iv = self.intern(&v);
            block_names.insert(ik, iv);
        }
        for (k, v) in raw.categories {
            let ik = self.intern(&k);
            let iv = self.intern(&v);
            categories.insert(ik, iv);
        }

        let lang = Language(id);
        if !self.order.contains(&lang) {
            self.order.push(lang);
        }
        self.packs.insert(
            id,
            LocalePack {
                id,
                code,
                native_name,
                strings,
                block_names,
                categories,
            },
        );
        Ok(lang)
    }

    fn load_dir(&mut self, dir: &Path) -> usize {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return 0;
        };
        let mut paths: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
            .collect();
        paths.sort();
        let mut count = 0;
        for path in paths {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if self.load_json_str(&content).is_ok() {
                    count += 1;
                }
            }
        }
        count
    }
}

fn candidate_locale_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(env_dir) = std::env::var("LOXONE_LOCALES_DIR") {
        if !env_dir.trim().is_empty() {
            dirs.push(PathBuf::from(env_dir));
        }
    }
    dirs.push(PathBuf::from("locales"));
    if let Some(cfg) = ConnectionProfile::default_config_path() {
        if let Some(parent) = cfg.parent() {
            dirs.push(parent.join("locales"));
        }
    }
    dirs
}

fn registry() -> &'static RwLock<I18nRegistry> {
    static REGISTRY: OnceLock<RwLock<I18nRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(I18nRegistry::new()))
}

fn intern_static_str(s: &str) -> &'static str {
    let mut reg = registry().write().unwrap();
    reg.intern(s)
}

/// Registers or updates a locale from a JSON string at runtime.
pub fn register_locale_json(json_str: &str) -> Result<Language, String> {
    let mut reg = registry().write().unwrap();
    reg.load_json_str(json_str)
}

/// Loads all `*.json` locale files from a directory at runtime.
pub fn load_locales_from_dir(dir: &Path) -> usize {
    let mut reg = registry().write().unwrap();
    reg.load_dir(dir)
}

/// Returns all currently registered languages in registration order.
pub fn available_languages() -> Vec<Language> {
    let reg = registry().read().unwrap();
    reg.order.clone()
}

/// Returns the currently active UI [`Language`].
pub fn current_language() -> Language {
    let reg = registry().read().unwrap();
    reg.active
}

/// Sets the active UI [`Language`].
pub fn set_language(lang: Language) {
    let mut reg = registry().write().unwrap();
    reg.active = lang;
}

pub fn default_language_config_path() -> Option<PathBuf> {
    ConnectionProfile::default_config_path().map(|p| p.with_file_name("language.txt"))
}

pub fn load_language_from(path: &Path) -> Language {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| Language::from_code(&s))
        .unwrap_or(Language::Pl)
}

pub fn save_language_to(path: &Path, lang: Language) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, format!("{}\n", lang.code()));
}

pub fn load_saved_language() -> Language {
    default_language_config_path()
        .as_deref()
        .map(load_language_from)
        .unwrap_or(Language::Pl)
}

pub fn save_language(lang: Language) {
    if let Some(path) = default_language_config_path() {
        save_language_to(&path, lang);
    }
}

/// Translates a UI string using the currently active [`Language`].
/// Falls back to `en` if the key is not translated in the active locale.
pub fn tr<'a>(en: &'a str) -> &'a str {
    tr_lang(current_language(), en)
}

/// Translates a UI string for the specified [`Language`].
pub fn tr_lang<'a>(lang: Language, en: &'a str) -> &'a str {
    let reg = registry().read().unwrap();
    if let Some(pack) = reg.packs.get(lang.0) {
        if let Some(&translated) = pack.strings.get(en) {
            return translated;
        }
    }
    en
}

/// Translates a parameterized template string using the currently active [`Language`]
/// and substitutes `{placeholder}` tokens with the given `(key, value)` pairs.
pub fn tr_fmt(en_template: &str, args: &[(&str, &str)]) -> String {
    tr_fmt_lang(current_language(), en_template, args)
}

/// Translates a parameterized template string for the specified [`Language`]
/// and substitutes `{placeholder}` tokens with the given `(key, value)` pairs.
pub fn tr_fmt_lang(lang: Language, en_template: &str, args: &[(&str, &str)]) -> String {
    let mut out = tr_lang(lang, en_template).to_string();
    for (k, v) in args {
        let token = format!("{{{}}}", k);
        out = out.replace(&token, v);
    }
    out
}

/// Returns the localized display name for a Loxone block or periphery `type_name`
/// in the currently active [`Language`].
pub fn block_display_name(type_name: &str) -> &str {
    block_display_name_lang(current_language(), type_name)
}

/// Returns the localized display name for a Loxone block or periphery `type_name`
/// in the specified [`Language`].
pub fn block_display_name_lang<'a>(lang: Language, type_name: &'a str) -> &'a str {
    let reg = registry().read().unwrap();
    if let Some(pack) = reg.packs.get(lang.0) {
        if let Some(&translated) = pack.block_names.get(type_name) {
            return translated;
        }
    }
    loxone_core::block_schema::lookup_block_schema(type_name)
        .map(|s| s.display_name_en)
        .unwrap_or(type_name)
}

/// Translates Loxone block category group names into the active [`Language`].
pub fn category_group_label(cat: &str) -> &str {
    category_group_label_lang(current_language(), cat)
}

/// Translates Loxone block category group names into the specified [`Language`].
pub fn category_group_label_lang<'a>(lang: Language, cat: &'a str) -> &'a str {
    let reg = registry().read().unwrap();
    if let Some(pack) = reg.packs.get(lang.0) {
        if let Some(&translated) = pack.categories.get(cat) {
            return translated;
        }
        if let Some(&translated) = pack.strings.get(cat) {
            return translated;
        }
    }
    cat
}

/// Returns `true` if `query_lower` matches the localized block name in any registered language.
pub fn block_matches_query(type_name: &str, query_lower: &str) -> bool {
    let reg = registry().read().unwrap();
    reg.packs.values().any(|pack| {
        pack.block_names
            .get(type_name)
            .is_some_and(|name| name.to_lowercase().contains(query_lower))
    })
}

/// Returns `true` if `query_lower` matches the localized category name in any registered language.
pub fn category_matches_query(cat: &str, query_lower: &str) -> bool {
    let reg = registry().read().unwrap();
    reg.packs.values().any(|pack| {
        pack.categories
            .get(cat)
            .or_else(|| pack.strings.get(cat))
            .is_some_and(|label| label.to_lowercase().contains(query_lower))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_default_is_polish_and_builtin_locales_work() {
        assert_eq!(Language::default(), Language::Pl);
        assert_eq!(Language::Pl.code(), "PL");
        assert_eq!(Language::Pl.native_name(), "Polski");
        assert_eq!(Language::En.code(), "EN");
        assert_eq!(Language::En.native_name(), "English");
        assert_eq!(Language::De.code(), "DE");
        assert_eq!(Language::De.native_name(), "Deutsch");

        let langs = available_languages();
        assert!(langs.contains(&Language::Pl));
        assert!(langs.contains(&Language::En));
        assert!(langs.contains(&Language::De));

        assert_eq!(tr_lang(Language::Pl, "+ Add Block"), "+ Dodaj blok");
        assert_eq!(tr_lang(Language::En, "+ Add Block"), "+ Add Block");
        assert_eq!(tr_lang(Language::De, "+ Add Block"), "+ Baustein");

        assert_eq!(
            block_display_name_lang(Language::Pl, "Alarm"),
            "System alarmowy"
        );
        assert_eq!(
            block_display_name_lang(Language::En, "Alarm"),
            "Burglar Alarm"
        );
        assert_eq!(
            block_display_name_lang(Language::De, "Alarm"),
            "Einbruchsalarmanlage"
        );

        assert_eq!(
            tr_fmt_lang(Language::Pl, "{count} project(s)", &[("count", "4")]),
            "4 proj."
        );
        assert_eq!(
            tr_fmt_lang(Language::En, "{count} project(s)", &[("count", "4")]),
            "4 project(s)"
        );
        assert_eq!(
            tr_fmt_lang(Language::De, "{count} project(s)", &[("count", "4")]),
            "4 Projekt(e)"
        );
    }

    #[test]
    fn test_dynamic_runtime_locale_registration() {
        let cs_json = r#"{
            "code": "CS",
            "native_name": "Čeština",
            "strings": {
                "+ Add Block": "+ Přidat blok",
                "{count} project(s)": "{count} projektů"
            },
            "block_names": {
                "Alarm": "Zabezpečovací systém"
            },
            "categories": {
                "Lighting": "Osvětlení"
            }
        }"#;

        let cs = register_locale_json(cs_json).expect("Czech locale JSON should register");
        assert_eq!(cs.code(), "CS");
        assert_eq!(cs.native_name(), "Čeština");
        assert!(available_languages().contains(&cs));
        assert_eq!(Language::from_code("cs"), cs);
        assert_eq!(tr_lang(cs, "+ Add Block"), "+ Přidat blok");
        assert_eq!(
            tr_fmt_lang(cs, "{count} project(s)", &[("count", "7")]),
            "7 projektů"
        );
        assert_eq!(
            block_display_name_lang(cs, "Alarm"),
            "Zabezpečovací systém"
        );
        assert_eq!(category_group_label_lang(cs, "Lighting"), "Osvětlení");
        assert!(block_matches_query("Alarm", "zabezpečovací"));
        assert!(category_matches_query("Lighting", "osvětlení"));
    }
}
