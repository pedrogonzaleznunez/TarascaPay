//! Configuración leída del entorno.

use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub host: String,
    pub jwt_secret: String,
    /// Vida del token de sesión, en horas.
    pub jwt_ttl_hours: i64,
    pub upload_dir: PathBuf,
    /// Tamaño máximo de un adjunto, en bytes.
    pub max_upload_bytes: usize,
    /// Orígenes permitidos por CORS. `*` habilita cualquiera (sólo desarrollo).
    pub cors_origins: Vec<String>,
    /// Carpeta con el build del frontend. Si existe, la API la sirve.
    pub web_dist: Option<PathBuf>,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| "falta la variable de entorno DATABASE_URL".to_string())?;

        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            // Sólo aceptable en desarrollo: los tokens dejan de valer al reiniciar.
            tracing::warn!(
                "JWT_SECRET no está definido; se genera uno efímero. \
                 Definilo en producción o las sesiones se invalidan en cada reinicio."
            );
            uuid::Uuid::new_v4().to_string()
        });

        if jwt_secret.len() < 16 {
            return Err("JWT_SECRET debe tener al menos 16 caracteres".to_string());
        }

        let port = parse_env("PORT", 3000u16)?;
        let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let jwt_ttl_hours = parse_env("JWT_TTL_HOURS", 24i64 * 30)?;
        let max_upload_bytes = parse_env("MAX_UPLOAD_BYTES", 10 * 1024 * 1024usize)?;

        let upload_dir = std::env::var("UPLOAD_DIR")
            .unwrap_or_else(|_| "./uploads".to_string())
            .into();

        let cors_origins = std::env::var("CORS_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:5173,http://127.0.0.1:5173".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let web_dist = std::env::var("WEB_DIST").ok().map(PathBuf::from);

        Ok(Config {
            database_url,
            port,
            host,
            jwt_secret,
            jwt_ttl_hours,
            upload_dir,
            max_upload_bytes,
            cors_origins,
            web_dist,
        })
    }
}

fn parse_env<T>(key: &str, default: T) -> Result<T, String>
where
    T: std::str::FromStr,
{
    match std::env::var(key) {
        Ok(raw) => raw
            .trim()
            .parse::<T>()
            .map_err(|_| format!("el valor de {key} ('{raw}') no es válido")),
        Err(_) => Ok(default),
    }
}
