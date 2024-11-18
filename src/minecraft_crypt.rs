use crate::util::copy_to_fixed_size;
use aes::Aes128;
use cfb8::cipher::NewCipher;
use cfb8::Cfb8;
use log::error;
use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey};
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use sha1::Digest;
use std::ops::Deref;
use std::process::exit;

pub struct RsaKeyPair {
    pub private: RsaPrivateKey,
    pub public: RsaPublicKey,
}

pub type Aes128Cfb = Cfb8<Aes128>;

pub fn generate_key_pair() -> RsaKeyPair {
    let bits = 1024;
    let private = RsaPrivateKey::new(&mut rand::thread_rng(), bits).unwrap_or_else(|error| {
        error!("Failed to generate key pair: {error}");
        exit(1);
    });
    let public = RsaPublicKey::from(&private);
    RsaKeyPair { public, private }
}

pub fn digest_data(
    id: &str,
    public_key: &RsaPublicKey,
    secret_key: &[u8],
) -> anyhow::Result<Vec<u8>> {
    Ok(digest_data_parts(vec![
        &id.chars().map(|c| c as u8).collect::<Vec<u8>>(),
        secret_key,
        public_key.to_public_key_der()?.as_bytes(),
    ]))
}

fn digest_data_parts(parts: Vec<&[u8]>) -> Vec<u8> {
    let mut hasher = sha1::Sha1::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().deref().to_owned()
}

pub fn decrypt_using_key(key: &RsaPrivateKey, data: Vec<u8>) -> anyhow::Result<Vec<u8>> {
    Ok(key.decrypt(Pkcs1v15Encrypt, &data)?)
}

pub fn get_cipher(key: &[u8]) -> anyhow::Result<Aes128Cfb> {
    Ok(Aes128Cfb::new_from_slices(key, key)?)
}
