use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params,
};

use crate::errors::UserError;

/// Hash a password using Argon2id with OWASP-recommended parameters
/// m_cost = 19456 KB (19 MB), t_cost = 2 iterations, p_cost = 1 thread
pub fn hash_password(password: &str) -> Result<String, UserError> {
    let salt = SaltString::generate(&mut OsRng);

    let params = Params::new(19456, 2, 1, None)
        .map_err(|e| UserError::CryptoError(format!("Error al crear parámetros: {}", e)))?;

    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| UserError::CryptoError(format!("Error al hashear contraseña: {}", e)))?;

    Ok(password_hash.to_string())
}

/// Verify a password against a stored Argon2id hash
pub fn verify_password(password: &str, password_hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(hash) => hash,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_correct_password() {
        let password = "TestPassword123!";
        let hash = hash_password(password).expect("Hashing should succeed");
        assert!(verify_password(password, &hash), "Should verify correct password");
    }

    #[test]
    fn test_verify_incorrect_password() {
        let password = "TestPassword123!";
        let wrong_password = "WrongPassword456!";
        let hash = hash_password(password).expect("Hashing should succeed");
        assert!(!verify_password(wrong_password, &hash), "Should reject wrong password");
    }

    #[test]
    fn test_verify_invalid_hash() {
        let result = verify_password("password", "$argon2id$invalid$hash");
        assert!(!result, "Should return false for invalid hash");
    }
}
