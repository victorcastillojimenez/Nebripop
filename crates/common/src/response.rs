use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub message: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            message: None,
        }
    }

    pub fn success_with_message(data: T, message: &str) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            message: Some(message.to_string()),
        }
    }

    pub fn error(error: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.to_string()),
            message: None,
        }
    }
}

/// Empty success response for operations that don't return data
#[derive(Debug, Serialize)]
pub struct EmptyResponse {
    pub success: bool,
    pub message: String,
}

impl EmptyResponse {
    pub fn ok() -> Self {
        Self {
            success: true,
            message: "Operación exitosa".to_string(),
        }
    }

    pub fn with_message(message: &str) -> Self {
        Self {
            success: true,
            message: message.to_string(),
        }
    }
}
