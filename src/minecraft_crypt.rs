use rand::SeedableRng;
use rsa::pkcs8::EncodePublicKey;
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use sha1::Digest;
use std::ops::Deref;

pub struct RsaKeyPair {
    pub private: RsaPrivateKey,
    pub public: RsaPublicKey,
}

pub fn generate_key_pair() -> RsaKeyPair {
    let mut rng = rand::rngs::StdRng::from_entropy();
    let bits = 1024;
    let private = RsaPrivateKey::new(&mut rng, bits).expect("Failed to generate private key");
    let public = RsaPublicKey::from(&private);
    RsaKeyPair { public, private }
}

pub fn digest_data(
    id: &str,
    public_key: RsaPublicKey,
    secret_key: Vec<u8>,
) -> anyhow::Result<Vec<u8>> {
    Ok(digest_data_parts(vec![
        &id.chars().map(|c| c as u8).collect::<Vec<u8>>(),
        &secret_key,
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

pub fn decrypt_using_key(key: RsaPrivateKey, data: Vec<u8>) -> anyhow::Result<Vec<u8>> {
    Ok(key.decrypt(Pkcs1v15Encrypt, &data)?)
}
