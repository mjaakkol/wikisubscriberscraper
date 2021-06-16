use std::{
    str::FromStr,
    collections::HashMap,
    fs::File,
    io::Write,
    path::Path,
    borrow::Cow,
    iter::Iterator
};

use reqwest::{
    Client,
    //Error
};

//use lazy_static::lazy_static;
use regex::Regex;

//use serde::Serialize;
//use serde::{Deserialize, Serialize};

use serde_json::value::Value;

use clap::{
    Arg,
    App
};

use scraper::{
    Html,
    Selector
};

use log::{info, debug, warn};

use tokio::fs::read_to_string;

async fn fetch(output_file: &Path) -> String {
    let europe = "https://en.wikipedia.org/w/api.php?action=parse&page=List_of_mobile_network_operators_of_Europe&prop=text&formatversion=2&format=json&disabletoc=true";

    let client = Client::new();

    let response = client
        .get(europe)
        .send()
        .await.unwrap()
        .json::<HashMap<String, Value>>()

        .await.unwrap();

    //println!("{:?}", response.keys());

    let j = serde_json::to_string(&response["parse"]["text"]).expect("Converting JSON into string failed");

    //let mut file = File::create(&output_file).expect("Opening file failed");
    //file.write_all(&j.as_bytes()).expect("Writing JSON file failed");

    j
}

fn parse_page(text: &str) {
    info!("Parse page");

    let fragment = Html::parse_fragment(text);
    let selector = Selector::parse("h2").unwrap();

    let h2 = fragment.select(&selector);

    let table_selector = Selector::parse("table").unwrap();
    let table = fragment.select(&table_selector);

    let th_selector = Selector::parse("th").unwrap();
    let td_selector = Selector::parse("td").unwrap();

    for (rows, country) in table.zip(h2) {
        println!("{:?}", country.text().collect::<Vec<_>>()[0]);

        // Handle Bulgaria thing
        let mut subscribers_unit = rows
                                            .select(&th_selector)
                                            .skip(3)
                                            .next()
                                            .unwrap()
                                            .text()
                                            .collect::<Vec<_>>();


        println!("{:?}", subscribers_unit);

        let mut pointer = rows
                                    .select(&td_selector)
                                    .skip(1);

        let operator = pointer
                                    .next()
                                    .unwrap()
                                    .text()
                                    .collect::<Vec<_>>()[0];

        println!("{:?}", operator);

        let technology = pointer.next().unwrap().text().collect::<Vec<_>>();
        println!("{:?}", technology);

        let mcc = pointer.skip(2).next().unwrap().text().collect::<Vec<_>>()[0];
        println!("{:?}", mcc);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
//async fn main()  {
    env_logger::init();

    let matches = App::new("Wiki mobile subscriber scraper")
                            .version("0.1")
                            .author("mikko.la.jaakkola@gmail.com")
                            .about("Donwloads mobile subscriber information into a file")
                            .arg(Arg::with_name("output")
                                .short("o")
                                .long("output")
                                .value_name("OUTPUT")
                                .help("Output file name"))
                                //.takes_value(true))
                            .arg(Arg::with_name("format")
                                .short("f")
                                .long("file")
                                .help("Output format for the file")
                                .value_name("OUTPUT_FORMAT")
                                .required(true)
                                .help("Output file format"))
                            .get_matches();

    // Gets a value for config if supplied by user, or defaults to "default.conf"
    let output = matches.value_of("output").unwrap_or("default.json");
    println!("Value for output file: {}", output);

    let output_file_path = Path::new(output);

    // Calling .unwrap() is safe here because "INPUT" is required (if "INPUT" wasn't
    // required we could have used an 'if let' to conditionally get the value)
    println!("Using input file: {}", matches.value_of("format").unwrap());

    let content = if output_file_path.exists() {
        read_to_string(&output_file_path).await.unwrap()
        //let deserialized: Value = serde_json::from_str(&serialized).unwrap();
    }
    else {
        fetch(&output_file_path).await
    };

    parse_page(&content);
    Ok(())
}
