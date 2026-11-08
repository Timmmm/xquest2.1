//! Persistent storage for the configuration, high scores and recorded demos.
//!
//! Natively these are files in the current directory, as in the original. On the web they
//! are kept in the browser's local storage.

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    pub fn load(name: &str) -> Option<Vec<u8>> {
        std::fs::read(name).ok()
    }

    pub fn save(name: &str, data: &[u8]) -> bool {
        std::fs::write(name, data).is_ok()
    }

    pub fn list(extension: &str) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(".")
            .map(|dir| {
                dir.filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .filter(|n| n.to_lowercase().ends_with(extension))
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    pub fn random_seed() -> u32 {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        (now.as_millis() as u64 ^ (now.as_nanos() as u64 >> 20)) as u32
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    const PREFIX: &str = "xquest/";

    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    pub fn load(name: &str) -> Option<Vec<u8>> {
        let hex = storage()?.get_item(&format!("{PREFIX}{name}")).ok()??;
        (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()).collect()
    }

    pub fn save(name: &str, data: &[u8]) -> bool {
        let hex: String = data.iter().map(|b| format!("{b:02x}")).collect();
        storage().map(|s| s.set_item(&format!("{PREFIX}{name}"), &hex).is_ok()).unwrap_or(false)
    }

    pub fn list(extension: &str) -> Vec<String> {
        let Some(s) = storage() else { return Vec::new() };
        let len = s.length().unwrap_or(0);
        let mut names: Vec<String> = (0..len)
            .filter_map(|i| s.key(i).ok().flatten())
            .filter_map(|k| k.strip_prefix(PREFIX).map(|n| n.to_string()))
            .filter(|n| n.to_lowercase().ends_with(extension))
            .collect();
        names.sort();
        names
    }

    pub fn random_seed() -> u32 {
        js_sys::Date::now() as u64 as u32
    }
}

pub use imp::{list, load, random_seed, save};
