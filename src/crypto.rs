use orion::aead;
use orion::errors::UnknownCryptoError;
use serde::ser::Serialize;
use serde::de::DeserializeOwned;
use serde_json;
use std::fs;
use std::path::Path;

type CryptoResult<T> = Result<T, UnknownCryptoError>;

pub fn encrypt(data: &[u8], pass: &str) -> CryptoResult<Vec<u8>> {
    let password = format!("{:0>32}", pass);
    let secret_key = aead::SecretKey::from_slice(password.as_bytes())?;
    aead::seal(&secret_key, data)
}

pub fn decrypt(data: &[u8], pass: &str) -> CryptoResult<Vec<u8>> {
    let password = format!("{:0>32}", pass);
    let secret_key = aead::SecretKey::from_slice(password.as_bytes())?;
    aead::open(&secret_key, data)
}

pub fn read_file_str<P>(path: P, pass: &str) -> CryptoResult<Option<String>>
where P: AsRef<Path> {
    if let Ok(contents_enc) = fs::read(path) {
        let contents_raw = decrypt(&contents_enc, pass)?;
        Ok(String::from_utf8(contents_raw).ok())
    } else {
        Ok(None)
    }
}

pub fn write_file_str<P>(path: P, val: &str, pass: &str) -> CryptoResult<()>
where P: AsRef<Path> {
    let contents_enc = encrypt(val.as_bytes(), pass)?;
    path.as_ref()
        .parent()
        .map(|parent| fs::create_dir_all(parent).unwrap_or_default());
    fs::write(path, contents_enc).unwrap();
    Ok(())
}

pub fn read_file_de<P, D>(path: P, pass: &str) -> CryptoResult<Option<D>>
where P: AsRef<Path>, D: DeserializeOwned {
    read_file_str(path, pass)
        .map(|res| res.map(|val| serde_json::from_str(&val).unwrap()))
}

pub fn write_file_ser<P, S>(path: P, val: S, pass: &str) -> CryptoResult<()>
where P: AsRef<Path>, S: Serialize {
    let val_str = serde_json::to_string(&val).unwrap();
    write_file_str(path, &val_str, pass)
}

pub fn encrypt_file<P, Q>(src: P, dest: Q, pass: &str) -> CryptoResult<()>
where P: AsRef<Path>, Q: AsRef<Path> {
    let contents_raw = fs::read(src).unwrap();
    let contents_enc = encrypt(&contents_raw, pass)?;
    dest.as_ref()
        .parent()
        .map(|parent| fs::create_dir_all(parent).unwrap_or_default());
    fs::write(dest, contents_enc).unwrap();
    Ok(())
}

pub fn decrypt_file<P, Q>(src: P, dest: Q, pass: &str) -> CryptoResult<()>
where P: AsRef<Path>, Q: AsRef<Path> {
    let contents_enc = fs::read(src).unwrap();
    let contents_raw = decrypt(&contents_enc, pass)?;
    dest.as_ref()
        .parent()
        .map(|parent| fs::create_dir_all(parent).unwrap_or_default());
    fs::write(dest, contents_raw).unwrap();
    Ok(())
}

#[cfg(test)]
mod test {
    // use once_cell::sync::Lazy;
    use orion::errors::UnknownCryptoError;
    // use std::sync::Mutex;

    // const CRYPTO_DIR: &'static str = "crypto-test-dir";
    // static DIR_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[test]
    fn should_decrypt_data_with_correct_password() {
        let (data, password) = ("contents", "1234");
        let data_enc = super::encrypt(data.as_bytes(), password).unwrap();
        let data_dec = super::decrypt(&data_enc, password).unwrap();
        assert_eq!(String::from_utf8(data_dec).unwrap(), data);
    }

    #[test]
    fn should_not_decrypt_data_with_incorrect_password() {
        let (data, password) = ("contents", "1234");
        let data_enc = super::encrypt(data.as_bytes(), password).unwrap();
        let error = super::decrypt(&data_enc, "12345").unwrap_err();
        assert_eq!(error, UnknownCryptoError);
    }

    // #[test]
    // fn should_decrypt
}
