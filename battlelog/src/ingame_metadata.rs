use crate::models::{IngameMetadataResponse};

impl IngameMetadataResponse {
    pub fn get_emblem_url(&self) -> Option<String> {
        if self.emblem_url.is_empty() {
            return None;
        }

        Some(self.emblem_url.replace(".dds", ".png"))
    }
}