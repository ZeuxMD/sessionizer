use argon2::password_hash::rand_core::OsRng;
use argon2::{password_hash::SaltString, Argon2, PasswordHasher, PasswordVerifier};
use rand::Rng;

pub fn hash_password(plain: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(plain.as_bytes(), &salt)
        .map_err(|e| e.to_string())?;
    Ok(hash.to_string())
}

pub fn verify_password(plain: &str, hash: &str) -> Result<bool, String> {
    let parsed_hash = argon2::PasswordHash::new(hash).map_err(|e| e.to_string())?;
    let argon2 = Argon2::default();
    Ok(argon2
        .verify_password(plain.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn generate_recovery_key() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    (0..16)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
