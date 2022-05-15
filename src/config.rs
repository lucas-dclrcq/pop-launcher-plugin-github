use serde::Deserialize;

use crate::launcher::config::find;

#[derive(Deserialize)]
pub struct PluginConfig {
    pub personal_access_token: String,
}

impl PluginConfig {
    pub fn load() -> Self {
        let path = find("github")
            .find(|path| path.exists())
            .expect("No config file");

        let config = std::fs::read_to_string(path).expect("Could not read config file");

        ron::from_str(&config).expect("Deserialization error")
    }
}
