use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN};
use ring::rand::{SecureRandom, SystemRandom};
use base64::Engine;
use std::env;

pub struct PasswordEncryption {
    key: LessSafeKey,
    rng: SystemRandom,
}

impl PasswordEncryption {
    pub fn new() -> Result<Self, String> {
        let key_hex = env::var("MF_ENCRYPTION_KEY")
            .map_err(|_| "MF_ENCRYPTION_KEY environment variable not set")?;
        
        let key_bytes = hex::decode(&key_hex)
            .map_err(|_| "Invalid encryption key format (must be hex)")?;
        
        if key_bytes.len() != 32 {
            return Err("Encryption key must be 32 bytes (64 hex characters)".into());
        }
        
        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(&key_bytes);
        
        let unbound_key = UnboundKey::new(&AES_256_GCM, &key_array)
            .map_err(|_| "Invalid encryption key")?;
        let key = LessSafeKey::new(unbound_key);
        
        Ok(Self {
            key,
            rng: SystemRandom::new(),
        })
    }
    
    pub fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        self.rng.fill(&mut nonce_bytes)
            .map_err(|_| "Failed to generate nonce")?;
        
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);
        let mut ciphertext = plaintext.as_bytes().to_vec();
        
        self.key.seal_in_place_append_tag(nonce, Aad::empty(), &mut ciphertext)
            .map_err(|_| "Encryption failed")?;
        
        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);
        
        Ok(base64::engine::general_purpose::STANDARD.encode(result))
    }
    
    pub fn decrypt(&self, encrypted: &str) -> Result<String, String> {
        let data = base64::engine::general_purpose::STANDARD.decode(encrypted)
            .map_err(|_| "Invalid base64")?;
        
        if data.len() < NONCE_LEN {
            return Err("Invalid encrypted data".into());
        }
        
        let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
        let nonce = Nonce::assume_unique_for_key(
            nonce_bytes.try_into().map_err(|_| "Invalid nonce")?
        );
        
        let mut plaintext = ciphertext.to_vec();
        let plaintext_bytes = self.key.open_in_place(nonce, Aad::empty(), &mut plaintext)
            .map_err(|_| "Decryption failed")?;
        
        String::from_utf8(plaintext_bytes.to_vec())
            .map_err(|_| "Invalid UTF-8".to_string())
    }
}

// Helper functions for easy use
pub fn encrypt_password(password: &str) -> Result<String, String> {
    let encryptor = PasswordEncryption::new()?;
    encryptor.encrypt(password)
}

pub fn decrypt_password(encrypted: &str) -> Result<String, String> {
    let encryptor = PasswordEncryption::new()?;
    encryptor.decrypt(encrypted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_encryption_roundtrip() {
        // Set test encryption key
        env::set_var("MF_ENCRYPTION_KEY", "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        
        let original = "test_password_123";
        let encrypted = encrypt_password(original).expect("Encryption should work");
        let decrypted = decrypt_password(&encrypted).expect("Decryption should work");
        
        assert_eq!(original, decrypted);
        assert_ne!(original, encrypted);
        assert!(encrypted.len() > original.len());
    }
    
    #[test]
    fn test_different_encryptions() {
        env::set_var("MF_ENCRYPTION_KEY", "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        
        let password = "same_password";
        let encrypted1 = encrypt_password(password).expect("First encryption should work");
        let encrypted2 = encrypt_password(password).expect("Second encryption should work");
        
        // Same password should encrypt to different values (due to random nonce)
        assert_ne!(encrypted1, encrypted2);
        
        // But both should decrypt to the same original
        assert_eq!(decrypt_password(&encrypted1).unwrap(), password);
        assert_eq!(decrypt_password(&encrypted2).unwrap(), password);
    }
}