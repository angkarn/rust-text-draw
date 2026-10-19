use std::{collections::HashMap, sync::Arc};

use image::RgbaImage;

#[cfg(feature = "decode")]
use crate::{Error, Result};

/// Images that `image` nodes can refer to by name (`src: "logo"`).
///
/// textplate never reads files itself; the caller decides where images come from.
#[derive(Clone, Debug, Default)]
pub struct Assets {
    images: HashMap<String, Arc<RgbaImage>>,
}

impl Assets {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: impl Into<String>, image: RgbaImage) -> &mut Self {
        self.images.insert(name.into(), Arc::new(image));
        self
    }

    /// Decodes a png, jpeg, webp, gif or bmp image and adds it.
    #[cfg(feature = "decode")]
    pub fn insert_encoded(&mut self, name: impl Into<String>, bytes: &[u8]) -> Result<&mut Self> {
        let name = name.into();
        let image = image::load_from_memory(bytes)
            .map_err(|source| Error::Decode {
                name: name.clone(),
                source,
            })?
            .into_rgba8();
        Ok(self.insert(name, image))
    }

    pub fn get(&self, name: &str) -> Option<&Arc<RgbaImage>> {
        self.images.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.images.contains_key(name)
    }
}
