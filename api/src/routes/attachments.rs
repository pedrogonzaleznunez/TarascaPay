//! Subida y descarga de comprobantes (fotos, PDFs).

use axum::body::Body;
use axum::extract::{Multipart, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use shared::{AttachmentResponse, MessageResponse};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::AttachmentRow;
use crate::repo;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/attachments", post(upload))
        .route(
            "/attachments/{attachment_id}",
            axum::routing::delete(delete_attachment),
        )
        .route("/attachments/{attachment_id}/file", axum::routing::get(serve))
}

/// Tipos aceptados y su extensión canónica.
const ALLOWED: &[(&str, &str)] = &[
    ("image/jpeg", "jpg"),
    ("image/png", "png"),
    ("image/webp", "webp"),
    ("image/gif", "gif"),
    ("image/heic", "heic"),
    ("application/pdf", "pdf"),
];

async fn upload(
    State(state): State<AppState>,
    user: AuthUser,
    mut multipart: Multipart,
) -> AppResult<Json<AttachmentResponse>> {
    let mut stored: Option<AttachmentResponse> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(format!("no se pudo leer el archivo: {e}")))?
    {
        if field.name() != Some("file") {
            continue;
        }

        let file_name = field
            .file_name()
            .map(sanitize_file_name)
            .unwrap_or_else(|| "comprobante".to_string());

        let declared = field.content_type().unwrap_or("").to_string();

        let bytes = field
            .bytes()
            .await
            .map_err(|e| AppError::bad_request(format!("no se pudo leer el archivo: {e}")))?;

        if bytes.is_empty() {
            return Err(AppError::bad_request("el archivo está vacío"));
        }
        if bytes.len() > state.config.max_upload_bytes {
            return Err(AppError::PayloadTooLarge(format!(
                "el archivo supera el máximo de {} MB",
                state.config.max_upload_bytes / (1024 * 1024)
            )));
        }

        // El tipo real se deduce del contenido, no de lo que declare el cliente:
        // así no se puede guardar cualquier cosa disfrazada de imagen.
        let mime = sniff_mime(&bytes).ok_or_else(|| {
            AppError::bad_request(format!(
                "formato no soportado{}. Se aceptan JPG, PNG, WebP, GIF, HEIC y PDF",
                if declared.is_empty() {
                    String::new()
                } else {
                    format!(" ({declared})")
                }
            ))
        })?;

        let extension = ALLOWED
            .iter()
            .find(|(m, _)| *m == mime)
            .map(|(_, ext)| *ext)
            .unwrap_or("bin");

        let id = Uuid::new_v4();
        let stored_name = format!("{id}.{extension}");
        let path = state.config.upload_dir.join(&stored_name);

        tokio::fs::write(&path, &bytes)
            .await
            .map_err(|e| AppError::internal(format!("no se pudo guardar el archivo: {e}")))?;

        let row = sqlx::query_as::<_, AttachmentRow>(
            "INSERT INTO attachments (id, uploaded_by, file_name, stored_name, mime_type, size_bytes)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, expense_id, uploaded_by, file_name, stored_name, mime_type,
                       size_bytes, created_at",
        )
        .bind(id)
        .bind(user.id)
        .bind(&file_name)
        .bind(&stored_name)
        .bind(mime)
        .bind(bytes.len() as i64)
        .fetch_one(&state.db)
        .await;

        let row = match row {
            Ok(row) => row,
            Err(err) => {
                // No dejamos el archivo huérfano si la fila no se pudo insertar.
                let _ = tokio::fs::remove_file(&path).await;
                return Err(err.into());
            }
        };

        stored = Some(row.into_response());
        break;
    }

    stored.ok_or_else(|| AppError::bad_request("falta el campo 'file' en el formulario"))
        .map(Json)
}

async fn serve(
    State(state): State<AppState>,
    user: AuthUser,
    Path(attachment_id): Path<Uuid>,
) -> AppResult<Response> {
    let row = load_visible(&state, attachment_id, user.id).await?;

    let path = state.config.upload_dir.join(&row.stored_name);
    let bytes = tokio::fs::read(&path).await.map_err(|_| {
        AppError::not_found("el archivo ya no está disponible en el almacenamiento")
    })?;

    let disposition = format!(
        "inline; filename=\"{}\"",
        row.file_name.replace('"', "").replace(['\r', '\n'], "")
    );

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, row.mime_type),
            (header::CONTENT_DISPOSITION, disposition),
            // El navegador no debe adivinar el tipo por su cuenta.
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_string()),
            (header::CACHE_CONTROL, "private, max-age=86400".to_string()),
        ],
        Body::from(bytes),
    )
        .into_response())
}

async fn delete_attachment(
    State(state): State<AppState>,
    user: AuthUser,
    Path(attachment_id): Path<Uuid>,
) -> AppResult<Json<MessageResponse>> {
    let row = load_visible(&state, attachment_id, user.id).await?;

    // Sólo quien lo subió puede borrarlo.
    if row.uploaded_by != user.id {
        return Err(AppError::forbidden("sólo quien subió el archivo puede borrarlo"));
    }

    sqlx::query("DELETE FROM attachments WHERE id = $1")
        .bind(attachment_id)
        .execute(&state.db)
        .await?;

    let _ = tokio::fs::remove_file(state.config.upload_dir.join(&row.stored_name)).await;

    Ok(Json(MessageResponse {
        message: "adjunto eliminado".to_string(),
    }))
}

/// Devuelve el adjunto sólo si el usuario puede verlo: o lo subió él, o está
/// asociado a un gasto al que tiene acceso.
async fn load_visible(
    state: &AppState,
    attachment_id: Uuid,
    user_id: Uuid,
) -> AppResult<AttachmentRow> {
    let row = sqlx::query_as::<_, AttachmentRow>(
        "SELECT id, expense_id, uploaded_by, file_name, stored_name, mime_type, size_bytes,
                created_at
         FROM attachments WHERE id = $1",
    )
    .bind(attachment_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::not_found("adjunto no encontrado"))?;

    if row.uploaded_by == user_id {
        return Ok(row);
    }

    match row.expense_id {
        Some(expense_id) => {
            repo::fetch_expense_for_user(&state.db, expense_id, user_id).await?;
            Ok(row)
        }
        None => Err(AppError::forbidden("no tenés acceso a este archivo")),
    }
}

/// Sustituye separadores y caracteres de control para que el nombre original
/// sea seguro de guardar y de devolver en una cabecera.
fn sanitize_file_name(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| match c {
            '/' | '\\' | '\0' | '"' | '\r' | '\n' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();

    let trimmed = cleaned.trim().trim_matches('.').to_string();
    if trimmed.is_empty() {
        "comprobante".to_string()
    } else {
        trimmed.chars().take(120).collect()
    }
}

/// Detecta el tipo real del archivo por sus bytes iniciales.
fn sniff_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() < 12 {
        return None;
    }

    // JPEG: FF D8 FF
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    // PNG: 89 50 4E 47 0D 0A 1A 0A
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some("image/png");
    }
    // GIF87a / GIF89a
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    // PDF: %PDF-
    if bytes.starts_with(b"%PDF-") {
        return Some("application/pdf");
    }
    // Contenedores ISO-BMFF / RIFF: 'ftyp' y 'WEBP' en el offset 8.
    if &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if brand == b"heic" || brand == b"heix" || brand == b"hevc" || brand == b"mif1" {
            return Some("image/heic");
        }
    }
    if bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_known_formats() {
        let mut png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        png.extend_from_slice(&[0u8; 8]);
        assert_eq!(sniff_mime(&png), Some("image/png"));

        let mut jpg = vec![0xFF, 0xD8, 0xFF, 0xE0];
        jpg.extend_from_slice(&[0u8; 12]);
        assert_eq!(sniff_mime(&jpg), Some("image/jpeg"));

        let mut pdf = b"%PDF-1.7".to_vec();
        pdf.extend_from_slice(&[0u8; 8]);
        assert_eq!(sniff_mime(&pdf), Some("application/pdf"));

        let mut webp = b"RIFF".to_vec();
        webp.extend_from_slice(&[0u8; 4]);
        webp.extend_from_slice(b"WEBP");
        assert_eq!(sniff_mime(&webp), Some("image/webp"));
    }

    #[test]
    fn rejects_disguised_files() {
        // Un HTML con nombre de imagen no debe pasar.
        let html = b"<!doctype html><script>alert(1)</script>".to_vec();
        assert_eq!(sniff_mime(&html), None);
        assert_eq!(sniff_mime(b"corto"), None);
    }

    #[test]
    fn file_names_cannot_escape_the_upload_dir() {
        assert_eq!(sanitize_file_name("../../etc/passwd"), "_.._etc_passwd");
        assert_eq!(sanitize_file_name("foto\n.png"), "foto_.png");
        assert_eq!(sanitize_file_name("   "), "comprobante");
        assert_eq!(sanitize_file_name("recibo.jpg"), "recibo.jpg");
    }
}
