pub struct Settings {
    pub application: ApplicationSettings,
}

#[derive(serde::Deserialize, Clone)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: String,
}
