<img src="images/logo.png" width="100%">

A simple and lightweight vault for secret storage written in Rust.

## Why?
Because I like simple stuff.

## Usage
```sh
vault get {path}
vault set {path} {contents}
vault rm {path}
vault ls {path-glob?}
vault fget {path-glob}
vault fset {path-glob}
vault fclr {path-glob}
vault crc [--force-update]
```

Password will be prompted securely for all the relevant operations. Given password is used to encrypt the first secret for a given file and should be used for further operations on the same file. The encrypted secrets are written out to file(s) in a folder in the current working directory. The generated .vlt files are portable.

## Building from source
Rust needs to be installed ([link](https://www.rust-lang.org/tools/install)). In the project directory, run the following command -

```sh
cargo build --release
```

The generated executable can be found in `./targets/release`.

## Running Tests
Execute the following command to run all tests -
```sh
cargo test
```

## Security considerations
Authenticated encryption is done using XChaCha20 and Poly1305 algorithms. The key size is 256 bits, so the maximum password length is 32 characters. Use a strong password to ensure maximum safety against dictionary attacks.

The password input is not displayed or stored in the terminal, but the secret outputs are NOT cleaned up on program end. However, if any secret is copied to clipboard, make sure it is cleaned after usage.

## Known issues
This program is not tested in, nor intended for windows as of now, so expect a lot of bugs there.

## Contributing
Feel free to raise issues and create PR if you feel something is missing or could be made better.
