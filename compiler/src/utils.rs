use std::fs;
use std::io::Write;

pub fn write_string_to_file(input: String, file_path: &str) -> std::io::Result<()> {
    let mut file = fs::File::create(file_path)?;
    file.write(input.as_bytes())?;
    Ok(())
}
