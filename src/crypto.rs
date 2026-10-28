use std::str::pattern::Pattern;

use crate::database::Database;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use rand::{distr::Alphanumeric, RngExt, rng};

fn generate_token() -> String {
    let length = 48;

    rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(password_hash.to_string())
}

fn verify_password(password: &str, parsed_hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(parsed_hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

pub fn login(db: &mut Database, username: String, password: String) -> Result<String, Box<dyn std::error::Error>>{
    let player_id = db.get_player_id_username(username)?;
    let pwd_hash = db.get_password_hash(player_id)?;
    if !verify_password(&password, &pwd_hash) {
        return Err("Bad Details".into());
    }
    let tok = generate_token();
    Ok(tok)
}