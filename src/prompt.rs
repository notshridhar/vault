use rpassword;
use std::io::{self, Write};

pub fn prompt_input_disappear(prompt: &str) -> io::Result<String> {
    let mut answer = String::new();
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut answer)?;
    print!("\x1b[1A\x1b[2K");
    io::stdout().flush().unwrap();
    Ok(answer)
}

pub fn prompt_secret_disappear(prompt: &str) -> io::Result<String> {
    let result = rpassword::prompt_password_stdout(prompt)?;
    print!("\x1b[1A\x1b[2K");
    io::stdout().flush().unwrap();
    Ok(result)
}
