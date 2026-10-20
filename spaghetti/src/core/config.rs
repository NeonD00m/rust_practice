use std::collections::HashSet;
use std::path::PathBuf;
use super::types::Language;

#[derive(Debug, Clone)]
pub struct Config {
    pub root_dir: PathBuf,
    pub no_ignore: bool,
    pub languages: HashSet<Language>,
    pub extensions: HashSet<String>,
    pub custom_extensions: HashSet<String>,
    pub max_depth: Option<usize>,
    pub color_distinct: bool,
    pub no_color: bool,
    pub raw: bool,
    pub json: bool,

    // Diagnostic settings
    pub warn_orphan: bool,
    pub warn_cycle: bool,
    pub warn_god: bool,
    pub warn_doc: bool,
    pub warn_error: bool,
    pub god_loc_threshold: usize,
    pub doc_ratio_threshold: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            root_dir: PathBuf::from("."),
            no_ignore: false,
            languages: HashSet::new(),
            extensions: HashSet::new(),
            custom_extensions: HashSet::new(),
            max_depth: None,
            color_distinct: false,
            no_color: false,
            raw: false,
            json: false,

            warn_orphan: true,
            warn_cycle: true,
            warn_god: true,
            warn_doc: false,
            warn_error: false,
            god_loc_threshold: 1000,
            doc_ratio_threshold: 5.0,
        }
    }
}

impl Config {
    pub fn is_language_enabled(&self, lang: Language) -> bool {
        if self.languages.is_empty() {
            true
        } else {
            self.languages.contains(&lang)
        }
    }

    pub fn is_ext_enabled(&self, ext: &str) -> bool {
        if self.extensions.is_empty() {
            true
        } else {
            self.extensions.contains(&ext.to_ascii_lowercase())
        }
    }

    pub fn is_custom_ext(&self, ext: &str) -> bool {
        self.custom_extensions.contains(&ext.to_ascii_lowercase())
    }
}
