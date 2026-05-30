use thiserror::Error;
use uuid::Uuid;

/// Domain errors for the payments module.
#[derive(Error, Debug)]
pub enum PaymentError {
    #[error("Error de Stripe: {0}")]
    StripeError(String),

    #[error("Firma de webhook de Stripe inválida")]
    InvalidSignature,

    #[error("Pago con ID {0} no encontrado")]
    NotFound(Uuid),

    #[error("El usuario {0} no tiene permisos para acceder a este pago")]
    Forbidden(Uuid),

    #[error("No puedes comprar tu propio anuncio")]
    SelfPurchase,

    #[error("El anuncio {0} no está disponible para la compra")]
    ListingNotAvailable(Uuid),

    #[error("Error de base de datos: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Error de validación: {0}")]
    ValidationError(String),
}
