use std::{collections::HashMap, path::Path, str::FromStr};

use reqwest::Client;

use serde_json::Value;

use clap::{App, Arg};

use log::{debug, error};

use tokio::fs::{read_to_string, write};

use thiserror::Error;

mod carriers;

async fn fetch(uri: &str, tag: &str) -> String {
    let cache_filename = &format!("cache_{}.html", tag);
    let cache_file_path = Path::new(&cache_filename);

    if cache_file_path.exists() {
        read_to_string(&cache_file_path).await.unwrap()
    } else {
        let client = Client::new();

        let response = client
            .get(uri)
            .send()
            .await
            .unwrap()
            .json::<HashMap<String, Value>>()
            .await
            .unwrap();

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

const JSON: &str = "json";
const CSV: &str = "csv";

#[derive(Debug)]
pub enum FileFormat {
    JSON,
    CSV,
}

impl FromStr for FileFormat {
    type Err = ScrapeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            JSON => Ok(FileFormat::JSON),
            CSV => Ok(FileFormat::JSON),
            _ => Err(ScrapeError::UnsupportedFileFormat),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let matches = App::new("Wiki mobile subscriber scraper")
        .version("0.1")
        .author("mikko.la.jaakkola@gmail.com")
        .about("Donwloads mobile subscriber information into a file")
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("OUTPUT")
                .takes_value(true)
                .help("Output file name"),
        )
        .arg(
            Arg::with_name("format")
                .short("f")
                .long("format")
                .help("Output format for the file")
                .value_name("OUTPUT_FORMAT")
                .possible_values(&[JSON, CSV])
                .help("Output file format"),
        )
        .get_matches();

    // Gets a value for config if supplied by user, or defaults to "default.conf"
    let output = matches.value_of("output").unwrap_or("default.json");
    debug!("Value for output file: {}", output);

    let mut output_file_path = Path::new(output).to_path_buf();

    let output_format = FileFormat::from_str(matches.value_of("format").unwrap_or(CSV))?;
    debug!("Using input file: {:?}", output_format);

    let carriers = carriers::Carriers::new();

    let serialized_carrier = carriers.parse(output_format, &mut output_file_path).await;

    write(&output_file_path, &serialized_carrier.as_bytes())
        .await
        .expect("Writing JSON file failed");

    Ok(())
}
