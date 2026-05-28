use thiserror::Error;

#[derive(Debug, Error)]
pub enum RatingError {
    #[error("Valoración no encontrada")]
    NotFound,

    #[error("Ya has valorado esta transacción")]
    AlreadyRated,

    #[error("Puntuación inválida: {0}. Debe estar entre 1 y 5")]
    InvalidScore(i16),

    #[error("La transacción no está completada")]
    TransactionNotCompleted,

    #[error("Error de validación: {0}")]
    ValidationError(String),

    #[error("Error de base de datos: {0}")]
    DatabaseError(String),
}
