//! Error type used across the crate.

use std::error::Error;
use std::fmt;
use std::io;
use std::string::FromUtf8Error;

/// Everything that can go wrong while encoding, decoding, or parsing arguments.
///
/// The enum is `#[non_exhaustive]`, so new variants can be added in later
/// releases without breaking code that matches on it. Add a wildcard arm when
/// matching.
#[derive(Debug)]
#[non_exhaustive]
pub enum HuffmanError {
    /// An underlying I/O operation failed. The original error is available
    /// through [`Error::source`].
    Io(io::Error),
    /// The input text or file was empty, so there is nothing to compress.
    EmptyFile,
    /// The input file was not valid UTF-8.
    InvalidUtf8,
    /// The file name did not have the extension the operation requires.
    InvalidExtension {
        /// The extension that was expected, for example `.txt`.
        expected: &'static str,
    },
    /// The compressed data is truncated, damaged, or not in this format.
    CorruptedData,
    /// The command line flag was not recognised.
    UnknownFlag(String),
    /// The program was started without any arguments.
    NoArguments,
    /// A flag was given without a file name after it.
    MissingFilename,
    /// The output file already exists. Existing files are never overwritten.
    OutputExists(String),
}

impl fmt::Display for HuffmanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HuffmanError::Io(e) => write!(f, "I/O error: {}", e),
            HuffmanError::EmptyFile => write!(f, "file is empty, nothing to compress"),
            HuffmanError::InvalidUtf8 => write!(f, "file is not valid UTF-8 text"),
            HuffmanError::InvalidExtension { expected } => {
                write!(f, "invalid file extension, expected '{}'", expected)
            }
            HuffmanError::CorruptedData => {
                write!(
                    f,
                    "failed to decompress: the file is corrupted or truncated"
                )
            }
            HuffmanError::UnknownFlag(flag) => {
                write!(f, "unknown flag '{}', use '-en' or '-den'", flag)
            }
            HuffmanError::NoArguments => write!(
                f,
                "missing arguments\nUsage:\n  huff -en <file.txt>\n  huff -den <file.en.huff>"
            ),
            HuffmanError::MissingFilename => write!(f, "missing filename after the flag"),
            HuffmanError::OutputExists(path) => {
                write!(
                    f,
                    "output file '{}' already exists, refusing to overwrite",
                    path
                )
            }
        }
    }
}

impl Error for HuffmanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            HuffmanError::Io(e) => Some(e),
            _ => None,
        }
    }
}

/// Lets `?` convert [`io::Error`] automatically.
impl From<io::Error> for HuffmanError {
    fn from(err: io::Error) -> Self {
        HuffmanError::Io(err)
    }
}

/// Lets `?` convert [`FromUtf8Error`] automatically.
impl From<FromUtf8Error> for HuffmanError {
    fn from(_: FromUtf8Error) -> Self {
        HuffmanError::InvalidUtf8
    }
}
