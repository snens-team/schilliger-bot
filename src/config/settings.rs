use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub token: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            token: "Your Token".to_string(),
        }
    }
}
