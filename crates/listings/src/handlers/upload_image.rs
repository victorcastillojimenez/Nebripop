use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::adapters::cloudinary::ImageStorageImpl;
use crate::adapters::listing_repository::ListingRepositoryImpl;
use crate::dtos::ListingImageResponseDto;
use crate::errors::map_listing_error;
use crate::usecases::upload_image_usecase;

use common::auth::AuthUser;
use common::errors::AppError;

/// POST /listings/:id/images
///
/// Uploads an image to a listing. Only the owner can upload.
/// Accepts multipart/form-data with field name "image".
///
/// Authentication: required (JWT Bearer token — must be the owner)
/// Errors:
/// - 400: invalid image type or size
/// - 403: not the owner
/// - 404: listing not found
/// - 422: too many images (max 10)
pub async fn upload_image_handler(
    State(repo): State<ListingRepositoryImpl>,
    State(image_storage): State<ImageStorageImpl>,
    auth_user: AuthUser,
    Path(listing_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<ListingImageResponseDto>), AppError> {
    // Extract the first image file from the multipart upload
    let (image_bytes, filename, content_type) = loop {
        let field = multipart
            .next_field()
            .await
            .map_err(|e| AppError::BadRequest(format!("Error al leer el multipart: {e}")))?
            .ok_or_else(|| AppError::BadRequest("No se encontró el campo 'image' en la petición".to_string()))?;

        if field.name() == Some("image") {
            let filename = field
                .file_name()
                .unwrap_or("image.jpg")
                .to_string();
            let content_type = field
                .content_type()
                .map(|ct| ct.to_string())
                .unwrap_or_else(|| "image/jpeg".to_string());
            let bytes = field.bytes().await.map_err(|e| {
                AppError::BadRequest(format!("Error al leer los datos de la imagen: {e}"))
            })?;

            break (bytes.to_vec(), filename, content_type);
        }
    };

    let result = upload_image_usecase::upload_image_usecase(
        &repo,
        &image_storage,
        listing_id,
        auth_user.id,
        image_bytes,
        &filename,
        &content_type,
    )
    .await
    .map_err(map_listing_error)?;

    Ok((StatusCode::CREATED, Json(result)))
}
