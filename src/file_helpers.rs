//! Small helpers for reading and writing files with crate errors.

use crate::error::HuffmanError;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};

/// Reads a whole file as UTF-8 text.
///
/// # Errors
///
/// * [`HuffmanError::Io`] if the file cannot be read.
/// * [`HuffmanError::InvalidUtf8`] if the contents are not valid UTF-8.
/// * [`HuffmanError::EmptyFile`] if the file is empty.
pub fn read_file(path: &str) -> Result<String, HuffmanError> {
    let bytes = fs::read(path)?;
    let content = String::from_utf8(bytes)?;
    if content.is_empty() {
        return Err(HuffmanError::EmptyFile);
    }
    Ok(content)
}

/// Creates a new file and writes `data` to it.
///
/// The file is created with `create_new`, so the check and the creation happen
/// in one atomic step and an existing file is never overwritten.
///
/// # Errors
///
/// * [`HuffmanError::OutputExists`] if a file already exists at `path`.
/// * [`HuffmanError::Io`] for any other I/O failure.
pub(crate) fn write_new_file(path: &str, data: &[u8]) -> Result<(), HuffmanError> {
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
