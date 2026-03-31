use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

#[derive(Clone)]
pub struct PasswordService {
    pepper: String,
}

impl PasswordService {
    pub fn new(pepper: String) -> Self {
        Self { pepper }
    }

    pub fn hash(&self, password: &str) -> Result<String, argon2::password_hash::Error> {
        let peppered = format!("{}{}", password, self.pepper);
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2.hash_password(peppered.as_bytes(), &salt)?;
        Ok(hash.to_string())
    }

    pub fn verify(&self, password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
        let peppered = format!("{}{}", password, self.pepper);
        let parsed_hash = PasswordHash::new(hash)?;
        let result = Argon2::default().verify_password(peppered.as_bytes(), &parsed_hash);
        Ok(result.is_ok())
    }
}
