use thiserror::Error;

#[derive(Debug, Error)]
pub enum FavoriteError {
    #[error("Favorito no encontrado")]
    NotFound,

    #[error("El favorito ya existe")]
    AlreadyExists,

    #[error("Error de base de datos: {0}")]
    DatabaseError(String),
}
