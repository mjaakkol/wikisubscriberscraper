use clap::{
    Arg,
    App
};

fn main() {
    let matches = App::new("Wiki mobile subscriber scraper")
                            .version("0.1")
                            .author("mikko.la.jaakkola@gmail.com")
                            .about("Donwloads mobile subscriber information into a file")
                            .arg(Arg::with_name("output")
                                .short("o")
                                .long("output")
                                .value_name("OUTPUT")
                                .help("Output file name")
                                .takes_value(true))
                            .arg(Arg::with_name("format")
                                .short("f")
                                .help("Output format for the file")
                                .value_name("OUTPUT_FORMAT")
                                .help("Output file format"))
                            .get_matches();
                            /*.arg(Arg::with_name("v")
                                .short("v")
                                .multiple(true)
                                .help("Sets the level of verbosity"))*/


    // Gets a value for config if supplied by user, or defaults to "default.conf"
    let output = matches.value_of("OUTPUT").unwrap_or("default.conf");
    println!("Value for output file: {}", output);

    // Calling .unwrap() is safe here because "INPUT" is required (if "INPUT" wasn't
    // required we could have used an 'if let' to conditionally get the value)
    println!("Using input file: {}", matches.value_of("OUTPUT_FORMAT").unwrap());

    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    /*match matches.occurrences_of("v") {
        0 => println!("No verbose info"),
        1 => println!("Some verbose info"),
        2 => println!("Tons of verbose info"),
        3 | _ => println!("Don't be crazy"),
    }*/
}
