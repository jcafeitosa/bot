use std::path::Path;
use std::sync::Once;

static DOTENV: Once = Once::new();

/// Loads `backend/.env` once per process (idempotent). Missing file is OK.
pub fn ensure_dotenv_loaded() {
    DOTENV.call_once(|| {
        if dotenvy::dotenv().is_err() {
            let backend_env = Path::new("backend/.env");
            if backend_env.is_file() {
                let _ = dotenvy::from_path(backend_env);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_dotenv_loaded_is_idempotent() {
        ensure_dotenv_loaded();
        ensure_dotenv_loaded();
    }
}
