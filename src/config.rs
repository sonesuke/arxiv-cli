#[derive(Debug, Clone)]
pub struct Config {
    pub headless: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { headless: true }
    }
}
