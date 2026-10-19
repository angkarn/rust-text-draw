use std::{collections::HashMap, sync::Arc};

use cosmic_text::fontdb;

use crate::{Error, Result};

/// Fonts available to templates, referred to by alias (`font: "title"`).
///
/// Every added font also takes part in fallback: characters missing from the
/// chosen font (another script, emoji, ...) are drawn with another added font.
#[derive(Clone, Default)]
pub struct Fonts {
    pub(crate) db: fontdb::Database,
    aliases: HashMap<String, String>,
    default: Option<String>,
}

impl Fonts {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a font file (TTF/OTF/TTC) under `alias`. The first font added is the default.
    pub fn add(&mut self, alias: impl Into<String>, data: impl Into<Vec<u8>>) -> Result<&mut Self> {
        let alias = alias.into();
        let source = fontdb::Source::Binary(Arc::new(data.into()));
        let ids = self.db.load_font_source(source);
        let family = ids
            .first()
            .and_then(|id| self.db.face(*id))
            .and_then(|face| face.families.first())
            .map(|(name, _)| name.clone())
            .ok_or_else(|| Error::InvalidFont(alias.clone()))?;
        if self.default.is_none() {
            self.default = Some(alias.clone());
        }
        self.aliases.insert(alias, family);
        Ok(self)
    }

    /// Makes `alias` the font used when a node sets no `font`.
    pub fn set_default(&mut self, alias: &str) -> Result<&mut Self> {
        if !self.aliases.contains_key(alias) {
            return Err(Error::UnknownFont {
                path: "Fonts::set_default".into(),
                font: alias.into(),
            });
        }
        self.default = Some(alias.to_string());
        Ok(self)
    }

    /// Family name behind an alias.
    pub fn family(&self, alias: &str) -> Option<&str> {
        self.aliases.get(alias).map(String::as_str)
    }

    pub fn default_alias(&self) -> Option<&str> {
        self.default.as_deref()
    }

    pub fn aliases(&self) -> impl Iterator<Item = &str> {
        self.aliases.keys().map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.aliases.is_empty()
    }
}

impl std::fmt::Debug for Fonts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fonts")
            .field("aliases", &self.aliases)
            .field("default", &self.default)
            .finish()
    }
}
