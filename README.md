<img src="images/logo.png" width="100%">

A simple and lightweight vault for secret storage written in Rust.

## Why?
Because I like simple stuff.

## Usage
```sh
$ vault --help
usage:
    vault [options] command args

commands:
    tui        starts vault in interactive mode
               this is the recommended way of using vault
               -----
    get        prints the secret contents at the given path
               usage: get <path>
               -----
    set        sets the secret contents at the given path
               creates new path if the path is not found
               replaces existing contents otherwise
               usage: set <path> <contents>
               -----
    rm         removes the given path and its contents
               usage: rm <path>
               -----
    ls         lists the paths matching the given pattern
               usage: ls <path-pattern>
               -----
    fget       decrypts paths matching the given pattern
               also works with non-unicode contents unlike get
               usage: fget <path-pattern>
               -----
    fset       encrypts paths matching the given pattern
               also works with non-unicode contents unlike set
               usage: fset <path-pattern>
               -----
    fclr       removes unlocked paths matching the given pattern
               does not affect the actual secret path or contents
               usage: fclr <path-pattern>
               -----
    crc        checks crc integrity for all paths and contents
               passing '--force-update' updates all checksums
               usage: crc [--force-update]
               -----
    zip        packs the encrypted contents for backup

options:
    --help     show this help message and exit
    --version  show the current version and exit
```

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

## Contributing
Feel free to raise issues and create PR if you feel something is missing or could be made better.
