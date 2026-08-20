//! Validaciones de entrada compartidas por los handlers.

use crate::error::{AppError, AppResult};

/// Normaliza y valida un email. Devuelve la versión en minúsculas.
pub fn email(raw: &str) -> AppResult<String> {
    let email = raw.trim().to_lowercase();

    if email.len() < 3 || email.len() > 254 {
        return Err(AppError::bad_request("el email no es válido"));
    }

    // Chequeo estructural mínimo: un `@`, algo antes, y un dominio con punto.
    let Some((local, domain)) = email.split_once('@') else {
        return Err(AppError::bad_request("el email no es válido"));
    };

    let looks_valid = !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !email.contains(' ');

    if !looks_valid {
        return Err(AppError::bad_request("el email no es válido"));
    }

    Ok(email)
}

pub fn password(raw: &str) -> AppResult<()> {
    if raw.chars().count() < 8 {
        return Err(AppError::bad_request(
            "la contraseña debe tener al menos 8 caracteres",
        ));
    }
    if raw.len() > 256 {
        return Err(AppError::bad_request("la contraseña es demasiado larga"));
    }
    Ok(())
}

pub fn non_empty(raw: &str, field: &str, max_len: usize) -> AppResult<String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(AppError::bad_request(format!("{field} no puede estar vacío")));
    }
    if value.chars().count() > max_len {
        return Err(AppError::bad_request(format!(
            "{field} no puede superar los {max_len} caracteres"
        )));
    }
    Ok(value.to_string())
}

/// Valida un código ISO-4217 de tres letras.
pub fn currency(raw: &str) -> AppResult<String> {
    let code = raw.trim().to_uppercase();
    if code.len() != 3 || !code.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(AppError::bad_request(
            "la moneda debe ser un código de 3 letras (ej. ARS, USD, EUR)",
        ));
    }
    Ok(code)
}

pub fn amount_cents(value: i64) -> AppResult<i64> {
    if value <= 0 {
        return Err(AppError::bad_request("el importe debe ser mayor a cero"));
    }
    // ~92 mil billones de centavos: cualquier cosa por encima es un error de carga.
    if value > 1_000_000_000_000_000 {
        return Err(AppError::bad_request("el importe es demasiado grande"));
    }
    Ok(value)
}

pub fn group_kind(raw: &str) -> AppResult<String> {
    const KINDS: [&str; 5] = ["trip", "home", "couple", "event", "other"];
    let kind = raw.trim().to_lowercase();
    if !KINDS.contains(&kind.as_str()) {
        return Err(AppError::bad_request(
            "el tipo de grupo no es válido (trip, home, couple, event, other)",
        ));
    }
    Ok(kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_reasonable_emails() {
        assert_eq!(email("  Hola@SoyGalo.com ").unwrap(), "hola@soygalo.com");
        assert!(email("a@b.co").is_ok());
    }

    #[test]
    fn rejects_malformed_emails() {
        for bad in ["", "sinarroba", "@dominio.com", "usuario@", "a@b", "a b@c.com"] {
            assert!(email(bad).is_err(), "debería rechazar '{bad}'");
        }
    }

    #[test]
    fn password_needs_eight_chars() {
        assert!(password("1234567").is_err());
        assert!(password("12345678").is_ok());
    }

    #[test]
    fn currency_is_normalized() {
        assert_eq!(currency("ars").unwrap(), "ARS");
        assert!(currency("PESOS").is_err());
        assert!(currency("A1S").is_err());
    }

    #[test]
    fn amounts_must_be_positive() {
        assert!(amount_cents(0).is_err());
        assert!(amount_cents(-100).is_err());
        assert!(amount_cents(1).is_ok());
    }

    #[test]
    fn non_empty_trims_and_limits() {
        assert_eq!(non_empty("  hola  ", "el nombre", 10).unwrap(), "hola");
        assert!(non_empty("   ", "el nombre", 10).is_err());
        assert!(non_empty("demasiado largo de verdad", "el nombre", 5).is_err());
    }
}
