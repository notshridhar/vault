use orion::aead;
use orion::errors::UnknownCryptoError;
use serde::ser::Serialize;
use serde::de::DeserializeOwned;
use serde_json;
use std::fs;
use std::path::Path;

pub fn encrypt(
    data: &[u8], password: &str
) -> Result<Vec<u8>, UnknownCryptoError> {
    let password = format!("{:0>32}", password);
    let secret_key = aead::SecretKey::from_slice(password.as_bytes())?;
    aead::seal(&secret_key, data)
}

pub fn decrypt(
    data: &[u8], password: &str
) -> Result<Vec<u8>, UnknownCryptoError> {
    let password = format!("{:0>32}", password);
    let secret_key = aead::SecretKey::from_slice(password.as_bytes())?;
    aead::open(&secret_key, data)
}

pub fn read_string_from_file<P: AsRef<Path>>(
    file_path: P, password: &str
) -> Result<Option<String>, UnknownCryptoError> {
    if let Ok(contents_enc) = fs::read(file_path) {
        let contents_raw = decrypt(&contents_enc, password)?;
        Ok(String::from_utf8(contents_raw).ok())        
    } else {
        Ok(None)
    }
}

pub fn write_str_to_file<P: AsRef<Path>>(
    file_path: P, contents: &str, password: &str
) -> Result<(), UnknownCryptoError> {
    let contents_enc = encrypt(contents.as_bytes(), password)?;
    file_path.as_ref().parent()
        .map(|parent| fs::create_dir_all(parent).unwrap_or_default());
    fs::write(file_path, contents_enc).unwrap();
    Ok(())
}

pub fn deserialize_from_file<P: AsRef<Path>, D: DeserializeOwned>(
    file_path: P, password: &str
) -> Result<Option<D>, UnknownCryptoError> {
    let contents = read_string_from_file(file_path, password)?;
    Ok(contents.map(|val| serde_json::from_str(&val).unwrap()))
}

pub fn serialize_to_file<P: AsRef<Path>, S: Serialize>(
    file_path: P, contents: S, password: &str
) -> Result<(), UnknownCryptoError> {
    let contents_str = serde_json::to_string(&contents).unwrap();
    write_str_to_file(file_path, &contents_str, password)
}

pub fn encrypt_file<P: AsRef<Path>>(
    src_path: P, dest_path: P, password: &str
) -> Result<(), UnknownCryptoError> {
    let contents_raw = fs::read(src_path).unwrap();
    let contents_enc = encrypt(&contents_raw, password)?;
    let dest_parent = dest_path.as_ref().parent().unwrap();
    fs::create_dir_all(dest_parent).unwrap();
    fs::write(dest_path, contents_enc).unwrap();
    Ok(())
}

pub fn decrypt_file<P: AsRef<Path>>(
    src_path: P, dest_path: P, password: &str
) -> Result<(), UnknownCryptoError> {
    let contents_enc = fs::read(src_path).unwrap();
    let contents_raw = decrypt(&contents_enc, password)?;
    dest_path.as_ref().parent()
        .map(|parent| fs::create_dir_all(parent).unwrap_or_default());
    fs::write(dest_path, contents_raw).unwrap();
    Ok(())
}
