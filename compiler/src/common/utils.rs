use std::fs;
use std::process::{Command, Stdio};
use std::io::Write;

pub fn write_string_to_file(input: String, file_path: &str) -> std::io::Result<()> {
    let mut file = fs::File::create(file_path)?;
    file.write(input.as_bytes())?;
    Ok(())
}

pub fn save_graph_pdf(input: &str, dot_file: &str, pdf_file: &str) -> std::io::Result<()> {
    write_string_to_file(
        input.to_string(),
        dot_file)?;

    let file = fs::File::create(pdf_file).unwrap();
    let stdio = Stdio::from(file);
    Command::new("dot")
        .arg(dot_file)
        .arg("-Tpdf")
        .stdout(stdio)
        .status()?;

    Ok(())
}
