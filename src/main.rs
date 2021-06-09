use clap::{
    Arg,
    App
};

fn scrape() {
    let europe = "https://en.wikipedia.org/w/rest.php/v1/page/List_of_mobile_network_operators_of_Europe";

    let echo_json: serde_json::Value = reqwest::Client::new()
        .post("https://jsonplaceholder.typicode.com/posts")
        .json(&serde_json::json!({
            "title": "Reqwest.rs",
            "body": "https://docs.rs/reqwest",
            "userId": 1
        }))
        .send()
        .await?
        .json()
        .await?;

}

fn main() {
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
                                .help("Output file format"))
                            .get_matches();
                            /*.arg(Arg::with_name("v")
                                .short("v")
                                .multiple(true)
                                .help("Sets the level of verbosity"))*/

    // Gets a value for config if supplied by user, or defaults to "default.conf"
    let output = matches.value_of("output").unwrap_or("default.json");
    println!("Value for output file: {}", output);

    // Calling .unwrap() is safe here because "INPUT" is required (if "INPUT" wasn't
    // required we could have used an 'if let' to conditionally get the value)
    println!("Using input file: {}", matches.value_of("format").unwrap());
}
