use orion::aead;
use orion::errors::UnknownCryptoError;
use serde_json;
use std::collections::HashMap;

type Dict = HashMap<String, String>;
type CryptoResult<T> = Result<T, UnknownCryptoError>;

pub fn encrypt_kv(map: &Dict, password: &str) -> CryptoResult<Vec<u8>> {
    let password = format!("{:0>32}", password);
    let secret_key = aead::SecretKey::from_slice(password.as_bytes())?;
    let map_str = serde_json::to_string(map).unwrap();
    let map_enc = aead::seal(&secret_key, map_str.as_bytes())?;
    Ok(map_enc)
}

pub fn decrypt_kv(data: &[u8], password: &str) -> CryptoResult<Dict> {
    let password = format!("{:0>32}", password);
    let secret_key = aead::SecretKey::from_slice(password.as_bytes())?;
    let map_dec = aead::open(&secret_key, data)?;
    let map_str = String::from_utf8(map_dec).unwrap();
    let map = serde_json::from_str::<Dict>(&map_str).unwrap();
    Ok(map)
}
