use crate::util::serde::{Serialize, Deserialize};
use orion::aead;
use orion::errors::UnknownCryptoError;
use std::fs;
use std::path::Path;

type CryptoResult<T> = Result<T, UnknownCryptoError>;

/// Encrypts a stream of bytes using the given password.
fn encrypt(data: &[u8], pass: &str) -> CryptoResult<Vec<u8>> {
    let password = format!("{:0>32}", pass);
    let secret_key = aead::SecretKey::from_slice(password.as_bytes())?;
    aead::seal(&secret_key, data)
}

/// Decrypts a stream of bytes using the given password.
/// - If the password does not match, returns `UnknownCryptoError`.
fn decrypt(data: &[u8], pass: &str) -> CryptoResult<Vec<u8>> {
    let password = format!("{:0>32}", pass);
    let secret_key = aead::SecretKey::from_slice(password.as_bytes())?;
    aead::open(&secret_key, data)
}

/// Writes out the serialized value to encrypted file using the given password.
pub fn write_file<P, S>(path: P, val: S, pass: &str) -> CryptoResult<()>
where P: AsRef<Path>, S: Serialize {
    let val_str = val.serialize();
    let contents_enc = encrypt(val_str.as_bytes(), pass)?;
    path.as_ref()
        .parent()
        .map(|parent| fs::create_dir_all(parent).unwrap());
    fs::write(path, contents_enc).unwrap();
    Ok(())
}

/// Reads the deserialized value of an encrypted file using the given password.
/// - If the file cannot be read or contains non-utf-8 values, returns `None`.
/// - If the password does not match, returns `UnknownCryptoError`.
pub fn read_file<P, D>(path: P, pass: &str) -> CryptoResult<Option<D>>
where P: AsRef<Path>, D: Deserialize {
    let result = if let Ok(contents_enc) = fs::read(path) {
        let contents_raw = decrypt(&contents_enc, pass)?;
        match String::from_utf8(contents_raw) {
            Ok(contents_str) => D::deserialize(&contents_str),
            Err(_) => None
        }
    } else {
        None
    };
    Ok(result)
}

/// Encrypts contents of src file into the dest file using the given password.
/// Doing so creates or updates the dest file, so the src file is not affected.
pub fn encrypt_file<P, Q>(src: P, dest: Q, pass: &str) -> CryptoResult<()>
where P: AsRef<Path>, Q: AsRef<Path> {
    let contents_raw = fs::read(src).unwrap();
    let contents_enc = encrypt(&contents_raw, pass)?;
    dest.as_ref()
        .parent()
        .map(|parent| fs::create_dir_all(parent).unwrap());
    fs::write(dest, contents_enc).unwrap();
    Ok(())
}

/// Decrypts contents of src file into the dest file using the given password.
/// Doing so creates or updates the dest file, so the src file is not affected.
/// - If the password does not match, returns `UnknownCryptoError`.
pub fn decrypt_file<P, Q>(src: P, dest: Q, pass: &str) -> CryptoResult<()>
where P: AsRef<Path>, Q: AsRef<Path> {
    let contents_enc = fs::read(src).unwrap();
    let contents_raw = decrypt(&contents_enc, pass)?;
    dest.as_ref()
        .parent()
        .map(|parent| fs::create_dir_all(parent).unwrap());
    fs::write(dest, contents_raw).unwrap();
    Ok(())
}

#[cfg(test)]
mod test {
    use once_cell::sync::Lazy;
    use orion::errors::UnknownCryptoError;
    use std::collections::HashMap;
    use std::fs;
    use std::panic;
    use std::path::Path;
    use std::sync::Mutex;

    const CRYPTO_DIR: &'static str = "crypto-test-dir";
    static DIR_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn run_test<T>(test: T) -> ()
    where T: FnOnce() -> () + panic::UnwindSafe {
        let lock = DIR_LOCK.lock().unwrap();
        fs::create_dir_all(CRYPTO_DIR).unwrap();
        let result = panic::catch_unwind(|| test());
        fs::remove_dir_all(CRYPTO_DIR).unwrap();
        drop(lock);
        assert!(result.is_ok())
    }

    #[test]
    fn should_encrypt_and_decrypt_data_with_same_pass() {
        let (data, pass) = ("contents", "1234");
        let data_enc = super::encrypt(data.as_bytes(), pass).unwrap();
        let data_dec = super::decrypt(&data_enc, pass).unwrap();
        assert_eq!(String::from_utf8(data_dec).unwrap(), data);
    }

    #[test]
    fn should_not_encrypt_and_decrypt_data_with_different_pass() {
        let (data, pass) = ("contents", "1234");
        let data_enc = super::encrypt(data.as_bytes(), pass).unwrap();
        let error = Err(UnknownCryptoError);
        assert_eq!(super::decrypt(&data_enc, "12345"), error);
    }

    #[test]
    fn should_read_non_existent_file_str_with_any_pass() {
        let file_path = Path::new(CRYPTO_DIR).join("key");
        let result = None as Option<String>;
        run_test(|| {
            assert_eq!(super::read_file(file_path, "1234"), Ok(result));
        })
    }

    #[test]
    fn should_write_and_read_file_str_with_same_pass() {
        let (data, pass) = ("contents".to_owned(), "1234");
        let file_path = Path::new(CRYPTO_DIR).join("key");
        run_test(|| {
            assert_eq!(super::write_file(&file_path, &data, pass), Ok(()));
            assert_eq!(super::read_file(file_path, pass), Ok(Some(data)));
        })
    }

    #[test]
    fn should_not_write_and_read_file_str_with_different_pass() {
        type Result = super::CryptoResult<Option<String>>;
        let (data, pass) = ("contents".to_owned(), "1234");
        let file_path = Path::new(CRYPTO_DIR).join("key");
        let error = Err(UnknownCryptoError) as Result;
        run_test(|| {
            assert_eq!(super::write_file(&file_path, &data, pass), Ok(()));
            assert_eq!(super::read_file(file_path, "12345"), error);
        })
    }

    #[test]
    fn should_write_and_read_file_serde_with_same_pass() {
        let data = HashMap::from([("key".to_owned(), 1234_u32)]);
        let pass = "1234";
        let file_path = Path::new(CRYPTO_DIR).join("key");
        run_test(|| {
            assert_eq!(super::write_file(&file_path, &data, pass), Ok(()));
            assert_eq!(super::read_file(file_path, pass), Ok(Some(data)));
        })
    }

    #[test]
    fn should_encrypt_and_decrypt_file_with_same_pass() {
        let (data, pass) = ("contents", "1234");
        let dec_path = Path::new(CRYPTO_DIR).join("key");
        let enc_path = Path::new(CRYPTO_DIR).join("key-enc");
        run_test(|| {
            fs::write(&dec_path, data).unwrap();
            assert!(super::encrypt_file(&dec_path, &enc_path, pass).is_ok());
            fs::remove_file(&dec_path).unwrap();
            assert!(super::decrypt_file(enc_path, &dec_path, pass).is_ok());
            assert_eq!(fs::read_to_string(dec_path).unwrap(), data);
        })
    }

    #[test]
    fn should_not_encrypt_and_decrypt_file_with_different_pass() {
        let (data, pass) = ("contents", "1234");
        let dec_path = Path::new(CRYPTO_DIR).join("key");
        let enc_path = Path::new(CRYPTO_DIR).join("key-enc");
        let error = Err(UnknownCryptoError);
        run_test(|| {
            fs::write(&dec_path, data).unwrap();
            assert!(super::encrypt_file(&dec_path, &enc_path, pass).is_ok());
            fs::remove_file(&dec_path).unwrap();
            assert_eq!(super::decrypt_file(enc_path, dec_path, "123"), error);
        })
    }
}
