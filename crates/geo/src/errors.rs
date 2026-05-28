use thiserror::Error;

#[derive(Debug, Error)]
pub enum GeoError {
    #[error("Coordenadas inválidas: {0}")]
    InvalidCoordinates(String),

    #[error("Radio excedido: máximo permitido 50000 metros (50 km)")]
    RadiusExceeded,

    #[error("Límite excedido: máximo permitido 100 resultados")]
    LimitExceeded,

    #[error("Error de base de datos: {0}")]
    DatabaseError(String),
}
