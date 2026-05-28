use thiserror::Error;

#[derive(Debug, Error)]
pub enum UserError {
    #[error("Usuario no encontrado")]
    NotFound,

    #[error("El email ya está registrado")]
    EmailAlreadyExists,

    #[error("Credenciales incorrectas")]
    InvalidCredentials,

    #[error("Token inválido o expirado")]
    InvalidToken,

    #[error("Error de base de datos: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Error de criptografía: {0}")]
    CryptoError(String),

    #[error("Error de validación: {0}")]
    ValidationError(String),
}
