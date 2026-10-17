//! Command line front end for `huffman-cli`.
//!
//! ```text
//! huff -en <file.txt>        writes <file>.en.huff
//! huff -den <file.en.huff>   writes <file>.dec.txt
//! ```
//!
//! This file only parses arguments and prints errors. All real work happens in
//! the library.

use huffman_cli::error::HuffmanError;
use huffman_cli::{file_helpers, huffman};
use std::error::Error;
use std::process;

/// A parsed command line request.
enum Command<'a> {
    Encode { filepath: &'a str },
    Decode { filepath: &'a str },
}

impl<'a> Command<'a> {
    /// Parses `args`, where `args[0]` is the program name, `args[1]` the flag
    /// and `args[2]` the file path.
    fn parse(args: &'a [String]) -> Result<Command<'a>, HuffmanError> {
        if args.len() < 2 {
            return Err(HuffmanError::NoArguments);
        }
        if args.len() < 3 {
            return Err(HuffmanError::MissingFilename);
        }

        let flag = args[1].as_str();
        let filepath = args[2].as_str();

        if flag == "-en" {
            Ok(Command::Encode { filepath })
        } else if flag == "-den" {
            Ok(Command::Decode { filepath })
        } else {
            Err(HuffmanError::UnknownFlag(flag.to_string()))
        }
    }

    /// Runs the command and returns any error to the caller. Printing is left
    /// to `main`, so each error is shown exactly once.
    fn run(self) -> Result<(), HuffmanError> {
        match self {
            Command::Encode { filepath } => {
                let stem = filepath
                    .strip_suffix(".txt")
                    .ok_or(HuffmanError::InvalidExtension { expected: ".txt" })?;
                let output_path = format!("{}.en.huff", stem);

                println!("Encoding '{}'...", filepath);
                let content = file_helpers::read_file(filepath)?;
                huffman::encode_to_file(&content, &output_path)?;
                println!("Success! File encoded to: {}", output_path);
            }

            Command::Decode { filepath } => {
                let stem =
                    filepath
                        .strip_suffix(".en.huff")
                        .ok_or(HuffmanError::InvalidExtension {
                            expected: ".en.huff",
                        })?;
                let output_path = format!("{}.dec.txt", stem);

                println!("Decoding '{}'...", filepath);
                huffman::decode_to_file(filepath, &output_path)?;
                println!("Success! File decoded to: {}", output_path);
            }
        }
        Ok(())
    }
}

fn run_cmd(args: &[String]) -> Result<(), HuffmanError> {
    let command = Command::parse(args)?;
    command.run()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if let Err(err) = run_cmd(&args) {
        eprintln!("Error: {}", err);

        // Print the underlying cause too, if there is one.
        let mut cause = err.source();
        while let Some(c) = cause {
            eprintln!("  caused by: {}", c);
            cause = c.source();
        }
        process::exit(1);
    }
}
