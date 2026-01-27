use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    let password = if args.len() > 1 {
        &args[1]
    } else {
        "admin123"
    };
    
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string();
    
    println!("Password: {}", password);
    println!("Hash: {}", password_hash);
    println!();
    println!("SQL to update admin user:");
    println!("UPDATE users SET password_hash = '{}' WHERE email = 'admin@example.com';", password_hash);
}
