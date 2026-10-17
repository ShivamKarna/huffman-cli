use crate::error::HuffmanError;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};

pub fn read_file(path: &str) -> Result<String, HuffmanError> {
    let bytes = fs::read(path)?;
    let content = String::from_utf8(bytes)?;
    if content.is_empty() {
        return Err(HuffmanError::EmptyFile);
    }
    Ok(content)
}

pub fn write_new_file(path: &str, data: &[u8]) -> Result<(), HuffmanError> {
    let result = OpenOptions::new().write(true).create_new(true).open(path);
    let mut file = match result {
        Ok(f) => f,
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            return Err(HuffmanError::OutputExists(path.to_string()));
        }
        Err(e) => return Err(HuffmanError::Io(e)),
    };

    file.write_all(data)?;
    Ok(())
}
