use std::error::Error;
use std::fmt;
use std::io;
use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum HuffmanError {
    Io(io::Error),
    EmptyFile,
    InvalidUtf8,
    InvalidExtension { expected: &'static str },
    CorruptedData,
    UnknownFlag(String),
    NoArguments,
    MissingFilename,
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
                "missing arguments\nUsage:\n  cargo run -- -en <file.txt>\n  cargo run -- -den <file.en.huff>"
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

// Lets `?` convert io::Error automatically
impl From<io::Error> for HuffmanError {
    fn from(err: io::Error) -> Self {
        HuffmanError::Io(err)
    }
}

// Lets `?` convert String::from_utf8 errors automatically
impl From<FromUtf8Error> for HuffmanError {
    fn from(_: FromUtf8Error) -> Self {
        HuffmanError::InvalidUtf8
    }
}
