use std::{
    str::FromStr,
    path::Path,
    iter::Iterator,
};

use reqwest::{
    Client,
    //Error
};

use lazy_static::lazy_static;
use regex::Regex;

use serde::Serialize;

use clap::{
    Arg,
    App
};


use scraper::{
    Html,
    Selector,
    ElementRef
};

use log::{info, debug, warn, error};

use tokio::fs::{
    read_to_string,
    write
};

use thiserror::Error;


#[derive(Error, Debug)]
enum ScrapeError {
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

}


#[derive(Serialize,Debug)]
struct CarrierInfo {
    operator: String,
    country: String,
    region: String,
    subscribers: f64,
    mccmnc: u32
}

impl CarrierInfo {
    fn new(operator: &str, country: &str, region: &str, subscribers: f64, mccmnc: u32) -> Self {
        Self {
            operator : operator.to_owned(),
            country : country.to_owned(),
            region : region.to_owned(),
            subscribers,
            mccmnc
        }
    }

    fn gnerate_csv_header() -> String {
        format!("Operator,Country,Region,Subscribers,MCCMNC\n")
    }

    fn to_csv(self) -> String {
        format!("{},{},{},{},{}", self.operator, self.country, self.country, self.subscribers, self.mccmnc)
    }

}

async fn fetch(uri: &str, tag: &str) -> String {
    let cache_filename = &format!("cache_{}.html", tag);
    let cache_file_path = Path::new(&cache_filename);

    if cache_file_path.exists() {
        read_to_string(&cache_file_path).await.unwrap()
    }
    else {
        let client = Client::new();

        let response = client
            .get(uri)
            .send()
            .await.unwrap()
            .text()
            .await.unwrap();

        response
    }
}

fn parse_header(rows: &ElementRef) -> Result<(usize, usize), ScrapeError> {
    let th_selector = Selector::parse("th").unwrap();

    let mut header_iter = rows.select(&th_selector).skip(3);

    if let Some(subscribers_unit_element) = header_iter.next() {
        let subscribers_unit = subscribers_unit_element.text().collect::<Vec<_>>();

        let multiplier = if subscribers_unit.len() > 1 {
            let unit = subscribers_unit[1].to_lowercase();

            if unit.contains("million") {
                1_000_000
            }
            else if unit.contains("thousand") {
                1_000
            }
            else if unit.contains("%") {
                warn!("% needs to be removed");
                return Err(ScrapeError::PercentageUnit);
            }
            else {
                return Err(ScrapeError::UnknownUnit(unit));
            }
        }
        else {
            // Just Subscribers refers to direct number mapping
            1
        };

        let n_remaining_items = header_iter.count();

        Ok((multiplier, n_remaining_items))
    }
    else {
        Err(ScrapeError::UnwrappingHeaderFailed)
    }
}

fn parse_carrier(carrier: &ElementRef, country: &str, region: &str, multiplier: usize, mcc: bool) -> Result<CarrierInfo, ScrapeError> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(\d|\.)+").unwrap();
    }

    let td_selector = Selector::parse("td").unwrap();

    // I don't want to deal with any percentage stuff now.
    if multiplier > 0 {
        let mut pointer = carrier
                            .select(&td_selector)
                            .skip(1);

        if let Some(operator_raw) = pointer.next() {
            let operator = operator_raw.text().collect::<Vec<_>>()[0];

            let _technology = pointer.next().unwrap().text().collect::<Vec<_>>();
            // TODO: Technology block parsing needs to be implemented at the later date

            if let Some(subscribers_text_option) = pointer.next() {
                let subscribers_text = subscribers_text_option.text().collect::<Vec<_>>();

                if subscribers_text.is_empty() {
                    return Err(ScrapeError::SubscriberValueEmpty);
                }

                let subscribers_text = subscribers_text[0].replace(" ","");

                if subscribers_text.chars().next().unwrap().is_numeric() {
                    let clean_text = RE.captures(&subscribers_text)
                        .expect("Parsing number failed")
                        .get(0)
                        .unwrap()
                        .as_str();

                    let subscribers = f64::from_str(clean_text).unwrap() * (multiplier as f64);

                    let mccmnc = if mcc {
                        let mcc = pointer.last().unwrap().text().collect::<Vec<_>>()[0];
                        if mcc.len() >= 5 {
                            u32::from_str(&mcc[..5]).unwrap_or(0)
                        }
                        else {
                            0
                        }
                    }
                    else {
                        0
                    };

                    debug!("{} {} Subscribers:{} MNC:{}", operator, country, subscribers, mccmnc);

                    return Ok(CarrierInfo::new(&operator, &country, &region, subscribers, mccmnc));

                }
                else {
                    warn!("Dropping {} due to invalid subscriber number {}", operator, subscribers_text);
                    return Err(ScrapeError::InvalidSubscriptions(operator.to_owned()));
                }
            }
            else {
                return Err(ScrapeError::SubscriberValueEmpty);
            }
        }
        return Err(ScrapeError::EmptyOperator);
    }
    Err(ScrapeError::PercentageUnit)
}


async fn parse_page(uri: &str, region: &str) -> Vec<CarrierInfo> {
    info!("Parse page");

    let text = fetch(uri, region).await;

    let mut carriers = Vec::with_capacity(128);

    let fragment = Html::parse_fragment(&text);

    let main_page = Selector::parse(".mw-parser-output").unwrap();
    let valid_subset = fragment.select(&main_page).next().unwrap();

    let selector = Selector::parse("h2").unwrap();

    let h2 = valid_subset.select(&selector);

    let table_selector = Selector::parse("table[class^=wikitable]").unwrap();
    let table = valid_subset.select(&table_selector);

    let tr_selector = Selector::parse("tr").unwrap();

    for (rows, country) in table.zip(h2) {
        let country = country.text().collect::<Vec<_>>()[0];

        if let Ok((multiplier, count)) = parse_header(&rows) {
            for row in rows.select(&tr_selector).skip(1) {
                match parse_carrier(&row, &country, &region, multiplier, count > 1) {
                    Ok(carrier) => carriers.push(carrier),
                    Err(err) => error!("country:{} {}", country, err)
                }
            }
        }
        else {
            error!("Failed to parse header. Dropping country {}", country);
        }
    }
    carriers
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let matches = App::new("Wiki mobile subscriber scraper")
                            .version("0.1")
                            .author("mikko.la.jaakkola@gmail.com")
                            .about("Donwloads mobile subscriber information into a file")
                            .arg(Arg::with_name("output")
                                .short("o")
                                .long("output")
                                .value_name("OUTPUT")
                                .takes_value(true)
                                .help("Output file name"))
                            .arg(Arg::with_name("format")
                                .short("f")
                                .long("file")
                                .help("Output format for the file")
                                .value_name("OUTPUT_FORMAT")
                                .help("Output file format"))
                            .get_matches();

    // Gets a value for config if supplied by user, or defaults to "default.conf"
    let output = matches.value_of("output").unwrap_or("default.json");
    debug!("Value for output file: {}", output);

    let output_file_path = Path::new(output);

    // Calling .unwrap() is safe here because "INPUT" is required (if "INPUT" wasn't
    // required we could have used an 'if let' to conditionally get the value)
    debug!("Using input file: {}", matches.value_of("format").unwrap());

    let world = [
       ("Europe", "https://en.wikipedia.org/wiki/List_of_mobile_network_operators_of_Europe"),
      ("Americas", "https://en.wikipedia.org/wiki/List_of_mobile_network_operators_of_the_Americas"),
      ("MEA", "https://en.wikipedia.org/wiki/List_of_mobile_network_operators_of_the_Middle_East_and_Africa"),
    ];

    let mut all_carriers = Vec::new();

    for (region, uri) in &world {
        all_carriers.append(
            &mut parse_page(&uri, region).await
        );
    }

    let serialized_carrier = serde_json::to_string(&all_carriers).expect("Serializing carriers failed");



    write(&output_file_path, &serialized_carrier.as_bytes()).await.expect("Writing JSON file failed");

    Ok(())
}
