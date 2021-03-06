use std::{path::Path, str::FromStr};

use clap::{App, Arg};

use log::{info, error};

use smol::fs;

use thiserror::Error;

mod carriers;

async fn fetch(uri: &str, tag: &str) -> String {
    let cache_filename = &format!("cache_{}.html", tag);
    let cache_file_path = Path::new(&cache_filename);

    if cache_file_path.exists() {
        fs::read_to_string(&cache_file_path).await.unwrap()
    } else {
        let response: serde_json::Value = surf::get(uri).recv_json().await.expect("Failed to fetch URI");
        // Wikipedia parser mangles quotation markers with backslashes and
        // doesn't like it at all
        response["parse"]["text"].to_string().replace("\\\"", "")
    }
}

#[derive(Error, Debug)]
pub enum ScrapeError {
    #[error("transparent")]
    IOError(#[from] std::io::Error),
    #[error("Unknown unit type {0}")]
    UnknownUnit(String),
    #[error("Percentage as unit. Drop the operator")]
    PercentageUnit,
    #[error("Invalid subscriber number for {0}")]
    InvalidSubscriptions(String),
    #[error("Subscriber value empty")]
    SubscriberValueEmpty,
    #[error("Empty operator")]
    EmptyOperator,
    #[error("Unwrapping header failed")]
    UnwrappingHeaderFailed,
    #[error("Unsupported file-format")]
    UnsupportedFileFormat,
}

const JSON_STR: &str = "json";
const CSV_STR: &str = "csv";

#[derive(Debug)]
pub enum FileFormat {
    JSON,
    CSV,
}

impl FromStr for FileFormat {
    type Err = ScrapeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            JSON_STR => Ok(FileFormat::JSON),
            CSV_STR => Ok(FileFormat::CSV),
            _ => Err(ScrapeError::UnsupportedFileFormat),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let matches = App::new("Wiki mobile subscriber scraper")
        .version("0.2")
        .author("mikko.la.jaakkola@gmail.com")
        .about("Donwloads mobile subscriber information into a file")
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                //.value_name("OUTPUT")
                .takes_value(true)
                .help("Output file name"),
        )
        .arg(
            Arg::with_name("format")
                .short("f")
                .long("format")
                .required(true)
                .help("Output format for the file")
                .takes_value(true)
                .possible_values(&[JSON_STR, CSV_STR])
                .help("Output file format"),
        )
        .get_matches();

    // Gets a value for config if supplied by user, or defaults to "default.conf"
    let output = matches.value_of("output").unwrap_or("default.json");
    info!("Value for output file: {}", output);

    let mut output_file_path = Path::new(output).to_path_buf();

    let format_str = matches.value_of("format").expect("Invalid value. This should not happen");

    info!("Output format at STR level: {:?}", format_str);

    let output_format = FileFormat::from_str(format_str)?;
    info!("Output format: {:?}", output_format);

    smol::block_on(async {
        let carriers = carriers::Carriers::new();

        let serialized_carrier = carriers.parse(output_format, &mut output_file_path).await;

        fs::write(&output_file_path, &serialized_carrier.as_bytes())
            .await
            .expect("Writing JSON file failed");

        Ok(())
    })
}
