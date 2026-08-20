//! Hash de contraseñas, emisión/validación de JWT y extractor de usuario.

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::state::AppState;

/// Contenido del JWT de sesión.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Id del usuario.
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
}

pub fn hash_password(plain: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(plain.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AppError::internal(format!("no se pudo hashear la contraseña: {e}")))
}

pub fn verify_password(plain: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(plain.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

pub fn issue_token(user_id: Uuid, secret: &str, ttl_hours: i64) -> Result<String, AppError> {
    let now = chrono::Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        iat: now.timestamp(),
        exp: (now + chrono::Duration::hours(ttl_hours)).timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::internal(format!("no se pudo firmar el token: {e}")))
}

pub fn verify_token(token: &str, secret: &str) -> Result<Uuid, AppError> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|_| AppError::unauthorized("sesión inválida o expirada"))?;

    Uuid::parse_str(&data.claims.sub).map_err(|_| AppError::unauthorized("token mal formado"))
}

/// Usuario autenticado. Se extrae del header `Authorization: Bearer <token>`.
///
/// Como alternativa acepta `?token=<jwt>` en la query string: las etiquetas
/// `<img src="...">` no pueden mandar headers, y así los comprobantes se
/// muestran sin exponerlos públicamente.
#[derive(Clone, Copy, Debug)]
pub struct AuthUser {
    pub id: Uuid,
}

impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let token = bearer_token(parts)
            .or_else(|| query_token(parts))
            .ok_or_else(|| AppError::unauthorized("falta el token de autenticación"))?;

        let id = verify_token(&token, &app_state.config.jwt_secret)?;
        Ok(AuthUser { id })
    }
}

fn bearer_token(parts: &Parts) -> Option<String> {
    let raw = parts
        .headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;

    raw.strip_prefix("Bearer ")
        .or_else(|| raw.strip_prefix("bearer "))
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

fn query_token(parts: &Parts) -> Option<String> {
    let query = parts.uri.query()?;
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == "token").then(|| urldecode(value))
    })
}

/// Decodificación mínima de percent-encoding para el parámetro `token`.
/// Un JWT sólo usa caracteres base64url y puntos, así que alcanza con esto.
fn urldecode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = String::with_capacity(value.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = &value[i + 1..i + 3];
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte as char);
                        i += 3;
                    }
                    Err(_) => {
                        out.push('%');
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(' ');
                i += 1;
            }
            b => {
                out.push(b as char);
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_roundtrip() {
        let hash = hash_password("tarasca-2026").unwrap();
        assert!(verify_password("tarasca-2026", &hash));
        assert!(!verify_password("otra-cosa", &hash));
    }

    #[test]
    fn hashes_are_salted() {
        let a = hash_password("misma-clave").unwrap();
        let b = hash_password("misma-clave").unwrap();
        assert_ne!(a, b, "dos hashes de la misma clave no deben coincidir");
    }

    #[test]
    fn token_roundtrip() {
        let id = Uuid::new_v4();
        let token = issue_token(id, "un-secreto-bien-largo", 1).unwrap();
        assert_eq!(verify_token(&token, "un-secreto-bien-largo").unwrap(), id);
    }

    #[test]
    fn token_rejects_wrong_secret() {
        let token = issue_token(Uuid::new_v4(), "un-secreto-bien-largo", 1).unwrap();
        assert!(verify_token(&token, "otro-secreto-distinto").is_err());
    }

    #[test]
    fn token_rejects_expired() {
        let token = issue_token(Uuid::new_v4(), "un-secreto-bien-largo", -1).unwrap();
        assert!(verify_token(&token, "un-secreto-bien-largo").is_err());
    }

    #[test]
    fn urldecode_handles_escapes() {
        assert_eq!(urldecode("abc.def"), "abc.def");
        assert_eq!(urldecode("a%2Eb"), "a.b");
    }
}
