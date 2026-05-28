use thiserror::Error;
use uuid::Uuid;

/// Domain errors for the listings module.
/// Each variant maps to a specific business rule violation or system error.
#[derive(Debug, Error)]
pub enum ListingError {
    /// Listing with the given ID was not found.
    #[error("Anuncio no encontrado: {0}")]
    NotFound(Uuid),

    /// The authenticated user is not the owner of this listing.
    #[error("No eres el propietario de este anuncio: {0}")]
    NotOwner(Uuid),

    /// The listing has already been sold and cannot be modified.
    #[error("El anuncio ya está vendido: {0}")]
    AlreadySold(Uuid),

    /// The listing already has the maximum number of images (10).
    #[error("El anuncio ya tiene el máximo de 10 imágenes")]
    TooManyImages,

    /// Input validation failed.
    #[error("Error de validación: {0}")]
    InvalidInput(String),

    /// Database operation failed.
    #[error("Error de base de datos: {0}")]
    Database(sqlx::Error),

    /// Image upload to Cloudinary (or local fallback) failed.
    #[error("Error al subir imagen: {0}")]
    ImageUpload(String),

    /// Image type is not allowed (only jpeg, png, webp).
    #[error("Tipo de imagen no permitido: {0}. Solo se aceptan JPEG, PNG y WebP")]
    InvalidImageType(String),

    /// Image file size exceeds the maximum (5MB).
    #[error("La imagen excede el tamaño máximo de 5MB")]
    ImageTooLarge,
}

impl From<sqlx::Error> for ListingError {
    fn from(e: sqlx::Error) -> Self {
        ListingError::Database(e)
    }
}

/// Maps a ListingError to an AppError for HTTP responses.
/// This centralised mapping avoids duplication across handlers.
pub fn map_listing_error(e: ListingError) -> common::errors::AppError {
    use common::errors::AppError;

    match e {
        ListingError::NotFound(id) => {
            AppError::NotFound(format!("Anuncio no encontrado: {}", id))
        }
        ListingError::NotOwner(_) => {
            AppError::Forbidden("No eres el propietario de este anuncio".to_string())
        }
        ListingError::AlreadySold(_) => {
            AppError::BadRequest("El anuncio ya está vendido".to_string())
        }
        ListingError::TooManyImages => {
            AppError::BadRequest("El anuncio ya tiene el máximo de 10 imágenes".to_string())
        }
        ListingError::InvalidInput(msg) => AppError::BadRequest(msg),
        ListingError::Database(e) => {
            tracing::error!("Database error: {}", e);
            AppError::Internal("Error interno del servidor".to_string())
        }
        ListingError::ImageUpload(msg) => {
            tracing::error!("Image upload error: {}", msg);
            AppError::Internal("Error al procesar la imagen".to_string())
        }
        ListingError::InvalidImageType(msg) => AppError::BadRequest(msg),
        ListingError::ImageTooLarge => {
            AppError::BadRequest("La imagen excede el tamaño máximo de 5MB".to_string())
        }
    }
}
