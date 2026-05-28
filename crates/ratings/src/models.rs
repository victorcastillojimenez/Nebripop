use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::RatingError;

/// Value Object que garantiza que la puntuación está siempre entre 1 y 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RatingScore(i16);

impl RatingScore {
    /// Crea un RatingScore validando que el valor esté en el rango 1-5.
    pub fn new(score: i16) -> Result<Self, RatingError> {
        if !(1..=5).contains(&score) {
            return Err(RatingError::InvalidScore(score));
        }
        Ok(Self(score))
    }

    pub fn value(&self) -> i16 {
        self.0
    }
}

/// Entidad que representa una valoración post-transacción.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rating {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub rater_id: Uuid,
    pub rated_id: Uuid,
    pub score: i16,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Rating {
    /// Crea una nueva valoración validando el score.
    pub fn new(
        id: Uuid,
        listing_id: Uuid,
        rater_id: Uuid,
        rated_id: Uuid,
        score: i16,
        comment: Option<String>,
        created_at: DateTime<Utc>,
    ) -> Result<Self, RatingError> {
        let _ = RatingScore::new(score)?;

        // Validar longitud del comentario si existe
        if let Some(ref c) = comment {
            if c.len() > 500 {
                return Err(RatingError::ValidationError(
                    "El comentario no puede exceder los 500 caracteres".to_string(),
                ));
            }
        }

        Ok(Self {
            id,
            listing_id,
            rater_id,
            rated_id,
            score,
            comment,
            created_at,
        })
    }
}

/// Resultado del cálculo de promedio de valoraciones.
#[derive(Debug, Clone, Serialize)]
pub struct RatingSummary {
    pub average: f64,
    pub total: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rating_score_new_valid() {
        let score = RatingScore::new(3).unwrap();
        assert_eq!(score.value(), 3);
    }

    #[test]
    fn test_rating_score_new_boundary_low() {
        let score = RatingScore::new(1).unwrap();
        assert_eq!(score.value(), 1);
    }

    #[test]
    fn test_rating_score_new_boundary_high() {
        let score = RatingScore::new(5).unwrap();
        assert_eq!(score.value(), 5);
    }

    #[test]
    fn test_rating_score_new_invalid_below() {
        let err = RatingScore::new(0).unwrap_err();
        assert!(matches!(err, RatingError::InvalidScore(0)));
    }

    #[test]
    fn test_rating_score_new_invalid_above() {
        let err = RatingScore::new(6).unwrap_err();
        assert!(matches!(err, RatingError::InvalidScore(6)));
    }

    #[test]
    fn test_rating_new_valid() {
        let rating = Rating::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            4,
            Some("Buen producto".to_string()),
            Utc::now(),
        )
        .unwrap();
        assert_eq!(rating.score, 4);
        assert_eq!(rating.comment.as_deref(), Some("Buen producto"));
    }

    #[test]
    fn test_rating_new_long_comment() {
        let long_comment = "a".repeat(501);
        let err = Rating::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            3,
            Some(long_comment),
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(err, RatingError::ValidationError(_)));
    }
}
