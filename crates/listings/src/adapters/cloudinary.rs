use async_trait::async_trait;
use std::path::Path;
use uuid::Uuid;

use crate::errors::ListingError;
use crate::ports::ImageStorage;

/// Maximum allowed image size: 5 MB.
const MAX_IMAGE_SIZE: usize = 5 * 1024 * 1024;

/// Allowed MIME types for image uploads.
const ALLOWED_MIME_TYPES: &[&str] = &["image/jpeg", "image/png", "image/webp"];

/// Image storage implementation with Cloudinary and local filesystem fallback.
///
/// If the `CLOUDINARY_URL` environment variable is set, images are uploaded to
/// Cloudinary with optimised transformations. Otherwise, they are saved locally
/// under `static/uploads/` and served via a static file handler.
#[derive(Debug, Clone)]
pub struct ImageStorageImpl {
    /// Whether Cloudinary is configured (based on env var presence).
    use_cloudinary: bool,
    /// Cloudinary URL (parsed from env).
    cloudinary_url: Option<String>,
    /// Cloudinary upload preset (from env or default).
    upload_preset: String,
    /// Base directory for local file storage.
    local_upload_dir: String,
    /// Base URL path for serving local files.
    local_serve_path: String,
}

impl Default for ImageStorageImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageStorageImpl {
    pub fn new() -> Self {
        let cloudinary_url = std::env::var("CLOUDINARY_URL").ok();
        let use_cloudinary = cloudinary_url.is_some();
        let upload_preset = std::env::var("CLOUDINARY_UPLOAD_PRESET")
            .unwrap_or_else(|_| "nebripop_upload".to_string());

        Self {
            use_cloudinary,
            cloudinary_url,
            upload_preset,
            local_upload_dir: "static/uploads".to_string(),
            local_serve_path: "/static/uploads".to_string(),
        }
    }

    /// Validate image bytes: check MIME type and size.
    fn validate_image(bytes: &[u8], content_type: &str) -> Result<(), ListingError> {
        // Check size
        if bytes.len() > MAX_IMAGE_SIZE {
            return Err(ListingError::ImageTooLarge);
        }

        // Check MIME type
        let normalized = content_type.to_lowercase();
        let allowed = ALLOWED_MIME_TYPES.iter().any(|&t| normalized == t);
        if !allowed {
            return Err(ListingError::InvalidImageType(content_type.to_string()));
        }

        Ok(())
    }

    /// Determine file extension from MIME type.
    fn extension_from_mime(mime: &str) -> &'static str {
        match mime.to_lowercase().as_str() {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/webp" => "webp",
            _ => "bin",
        }
    }
}

#[async_trait]
impl ImageStorage for ImageStorageImpl {
    async fn upload(&self, bytes: Vec<u8>, _filename: &str, content_type: &str) -> Result<String, ListingError> {
        Self::validate_image(&bytes, content_type)?;

        if self.use_cloudinary {
            // Cloudinary upload
            self.cloudinary_upload(bytes, content_type).await
        } else {
            // Local filesystem fallback
            self.local_upload(bytes, content_type)
        }
    }

    async fn delete(&self, url: &str) -> Result<(), ListingError> {
        if self.use_cloudinary {
            self.cloudinary_delete(url).await
        } else {
            self.local_delete(url)
        }
    }

    fn get_optimized_url(&self, url: &str) -> String {
        if self.use_cloudinary && url.contains("cloudinary") {
            // Insert transformation parameters into Cloudinary URL.
            // Cloudinary URL format: https://res.cloudinary.com/<cloud>/image/upload/<public_id>
            // We insert after "upload": c_fill,g_auto,w_800,h_600,f_webp,q_auto
            url.replace("/upload/", "/upload/c_fill,g_auto,w_800,h_600,f_webp,q_auto/")
        } else {
            // Local URL: no transformation needed.
            url.to_string()
        }
    }
}

// ───────── Cloudinary implementation ─────────

impl ImageStorageImpl {
    async fn cloudinary_upload(&self, bytes: Vec<u8>, content_type: &str) -> Result<String, ListingError> {
        // Build a multipart form-data request to Cloudinary's upload API.
        let cloud_url = self.cloudinary_url.as_deref().unwrap_or("");
        // Parse cloudinary:// URL format: cloudinary://api_key:api_secret@cloud_name
        let (api_key, api_secret, cloud_name) = parse_cloudinary_url(cloud_url)?;

        let ext = Self::extension_from_mime(content_type);
        let public_id = format!("listings/{}", Uuid::new_v4());

        let upload_url = format!(
            "https://api.cloudinary.com/v1_1/{}/image/upload",
            cloud_name
        );

        // Build multipart body
        let boundary = format!("----{}", Uuid::new_v4());
        let mut body = Vec::new();

        // api_key field
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"api_key\"\r\n\r\n");
        body.extend_from_slice(api_key.as_bytes());
        body.extend_from_slice(b"\r\n");

        // public_id field
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"public_id\"\r\n\r\n");
        body.extend_from_slice(public_id.as_bytes());
        body.extend_from_slice(b"\r\n");

        // timestamp field
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"timestamp\"\r\n\r\n");
        body.extend_from_slice(timestamp.to_string().as_bytes());
        body.extend_from_slice(b"\r\n");

        // upload_preset
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"upload_preset\"\r\n\r\n");
        body.extend_from_slice(self.upload_preset.as_bytes());
        body.extend_from_slice(b"\r\n");

        // signature
        let sig_string = format!("public_id={public_id}&timestamp={timestamp}{api_secret}");
        let signature = md5_hash(&sig_string);
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"signature\"\r\n\r\n");
        body.extend_from_slice(signature.as_bytes());
        body.extend_from_slice(b"\r\n");

        // file field
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"file\"; filename=\"img.{ext}\"\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
        body.extend_from_slice(&bytes);
        body.extend_from_slice(b"\r\n");

        // End boundary
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

        let client = reqwest::Client::new();
        let response = client
            .post(&upload_url)
            .header("Content-Type", format!("multipart/form-data; boundary={boundary}"))
            .body(body)
            .send()
            .await
            .map_err(|e| ListingError::ImageUpload(format!("Cloudinary request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(ListingError::ImageUpload(format!(
                "Cloudinary returned {status}: {text}"
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ListingError::ImageUpload(format!("Failed to parse Cloudinary response: {e}")))?;

        let url = json["secure_url"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| json["url"].as_str().map(|s| s.to_string()))
            .ok_or_else(|| ListingError::ImageUpload("Cloudinary response missing URL".to_string()))?;

        Ok(url)
    }

    async fn cloudinary_delete(&self, url: &str) -> Result<(), ListingError> {
        let cloud_url = self.cloudinary_url.as_deref().unwrap_or("");
        let (_api_key, api_secret, cloud_name) = parse_cloudinary_url(cloud_url)?;

        // Extract public_id from URL
        let public_id = extract_cloudinary_public_id(url);
        let delete_url = format!(
            "https://api.cloudinary.com/v1_1/{}/image/destroy",
            cloud_name
        );

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let sig_string = format!("public_id={public_id}&timestamp={timestamp}{api_secret}");
        let signature = md5_hash(&sig_string);

        let client = reqwest::Client::new();
        let params = [
            ("public_id", public_id.as_str()),
            ("signature", signature.as_str()),
            ("api_key", &_api_key),
            ("timestamp", &timestamp.to_string()),
        ];

        let response = client
            .post(&delete_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| ListingError::ImageUpload(format!("Cloudinary delete failed: {e}")))?;

        if !response.status().is_success() {
            tracing::warn!("Cloudinary delete returned {}", response.status());
        }

        Ok(())
    }
}

// ───────── Local filesystem fallback ─────────

impl ImageStorageImpl {
    fn local_upload(&self, bytes: Vec<u8>, content_type: &str) -> Result<String, ListingError> {
        let ext = Self::extension_from_mime(content_type);
        let filename = format!("{}.{}", Uuid::new_v4(), ext);
        let relative_path = format!("{}/{}", self.local_upload_dir, filename);

        // Create directory if it doesn't exist
        if let Some(parent) = Path::new(&relative_path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ListingError::ImageUpload(format!("Failed to create upload directory: {e}"))
            })?;
        }

        std::fs::write(&relative_path, &bytes).map_err(|e| {
            ListingError::ImageUpload(format!("Failed to write file: {e}"))
        })?;

        Ok(format!("{}/{}", self.local_serve_path, filename))
    }

    fn local_delete(&self, url: &str) -> Result<(), ListingError> {
        // Remove the serve path prefix to get the relative file path
        if let Some(relative) = url.strip_prefix(&self.local_serve_path) {
            let full_path = format!("{}{}", self.local_upload_dir, relative);
            let _ = std::fs::remove_file(&full_path);
        }
        Ok(())
    }
}

// ───────── Helper functions ─────────

/// Parse cloudinary:// URL: cloudinary://api_key:api_secret@cloud_name
fn parse_cloudinary_url(url: &str) -> Result<(String, String, String), ListingError> {
    // Remove the scheme
    let without_scheme = url
        .strip_prefix("cloudinary://")
        .or_else(|| url.strip_prefix("cloudinary:"))
        .unwrap_or(url);

    let parts: Vec<&str> = without_scheme.splitn(2, '@').collect();
    if parts.len() != 2 {
        return Err(ListingError::ImageUpload(
            "Invalid CLOUDINARY_URL format. Expected: cloudinary://api_key:api_secret@cloud_name"
                .to_string(),
        ));
    }

    let credentials: Vec<&str> = parts[0].splitn(2, ':').collect();
    if credentials.len() != 2 {
        return Err(ListingError::ImageUpload(
            "Invalid CLOUDINARY_URL: missing api_key:api_secret".to_string(),
        ));
    }

    Ok((
        credentials[0].to_string(),
        credentials[1].to_string(),
        parts[1].to_string(),
    ))
}

/// Extract the public_id from a Cloudinary URL.
/// URL format: https://res.cloudinary.com/<cloud>/image/upload/v1234/<public_id>.<ext>
fn extract_cloudinary_public_id(url: &str) -> String {
    // Find "/upload/" in the URL
    if let Some(pos) = url.find("/upload/") {
        let after_upload = &url[pos + 8..]; // after "/upload/"
        // Remove version prefix (e.g., "v1234/")
        let without_version = if after_upload.starts_with('v') {
            if let Some(slash_pos) = after_upload.find('/') {
                &after_upload[slash_pos + 1..]
            } else {
                after_upload
            }
        } else {
            after_upload
        };
        // Remove file extension
        if let Some(dot_pos) = without_version.rfind('.') {
            without_version[..dot_pos].to_string()
        } else {
            without_version.to_string()
        }
    } else {
        url.to_string()
    }
}

/// Simple MD5 hash (hex string) for Cloudinary signature.
fn md5_hash(input: &str) -> String {
    use md5::{Md5, Digest};
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    format!("{:x}", digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cloudinary_url_valid() {
        let url = "cloudinary://abc123:my_secret@mycloud";
        let (key, secret, cloud) = parse_cloudinary_url(url).unwrap();
        assert_eq!(key, "abc123");
        assert_eq!(secret, "my_secret");
        assert_eq!(cloud, "mycloud");
    }

    #[test]
    fn test_parse_cloudinary_url_invalid() {
        assert!(parse_cloudinary_url("invalid").is_err());
        assert!(parse_cloudinary_url("cloudinary://onlykey").is_err());
    }

    #[test]
    fn test_extract_public_id() {
        let url = "https://res.cloudinary.com/mycloud/image/upload/v1234/listings/abc123.jpg";
        assert_eq!(extract_cloudinary_public_id(url), "listings/abc123");
    }

    #[test]
    fn test_validate_image_size_ok() {
        let bytes = vec![0u8; 1024]; // 1KB
        assert!(ImageStorageImpl::validate_image(&bytes, "image/jpeg").is_ok());
    }

    #[test]
    fn test_validate_image_size_too_large() {
        let bytes = vec![0u8; MAX_IMAGE_SIZE + 1];
        assert!(ImageStorageImpl::validate_image(&bytes, "image/jpeg").is_err());
    }

    #[test]
    fn test_validate_image_invalid_mime() {
        let bytes = vec![0u8; 1024];
        assert!(ImageStorageImpl::validate_image(&bytes, "image/gif").is_err());
        assert!(ImageStorageImpl::validate_image(&bytes, "application/pdf").is_err());
    }

    #[test]
    fn test_extension_from_mime() {
        assert_eq!(ImageStorageImpl::extension_from_mime("image/jpeg"), "jpg");
        assert_eq!(ImageStorageImpl::extension_from_mime("image/png"), "png");
        assert_eq!(ImageStorageImpl::extension_from_mime("image/webp"), "webp");
        assert_eq!(ImageStorageImpl::extension_from_mime("image/gif"), "bin");
    }

    #[test]
    fn test_get_optimized_url_cloudinary() {
        let storage = ImageStorageImpl {
            use_cloudinary: true,
            cloudinary_url: Some("cloudinary://k:s@c".to_string()),
            upload_preset: "test_preset".to_string(),
            local_upload_dir: "static/uploads".to_string(),
            local_serve_path: "/static/uploads".to_string(),
        };
        let url = "https://res.cloudinary.com/mycloud/image/upload/v1234/img.jpg";
        let optimized = storage.get_optimized_url(url);
        assert!(optimized.contains("c_fill,g_auto,w_800,h_600,f_webp,q_auto"));
    }

    #[test]
    fn test_get_optimized_url_local() {
        let storage = ImageStorageImpl {
            use_cloudinary: false,
            cloudinary_url: None,
            upload_preset: "test_preset".to_string(),
            local_upload_dir: "static/uploads".to_string(),
            local_serve_path: "/static/uploads".to_string(),
        };
        let url = "/static/uploads/img.jpg";
        let optimized = storage.get_optimized_url(url);
        assert_eq!(optimized, url); // No transformation for local files
    }
}
