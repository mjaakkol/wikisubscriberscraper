use std::{
    str::FromStr,
    fmt,
    iter::Iterator,
    path::PathBuf
};

use serde::Serialize;

use scraper::{
    Html,
    Selector,
    ElementRef
};

use lazy_static::lazy_static;

use regex::Regex;

use log::{info, debug, warn, error};

use crate::ScrapeError;
use crate::FileFormat;

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
        lazy_static! {
            static ref RE_TRIM: Regex = Regex::new(r"(\\+n| *\(\S*)$").unwrap();
        }

        let clean_country = RE_TRIM.replace_all(country,"");
        let clean_operator = RE_TRIM.replace_all(operator,"");

        Self {
            operator : clean_operator.to_string(),
            country : clean_country.to_string(),
            region : region.to_owned(),
            subscribers,
            mccmnc
        }
    }

    fn gnerate_csv_header() -> String {
        format!("Operator,Country,Region,Subscribers,MCCMNC\n")
    }

    fn check_string(value: &str) -> String {
        if let Some(_) = value.find(",") {
            format!("\"{}\"", value)
        }
        else {
            value.to_owned()
        }
    }
}

impl fmt::Display for CarrierInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        write!(f, "{},{},{},{},{}",
            CarrierInfo::check_string(&self.operator),
            CarrierInfo::check_string(&self.country),
            CarrierInfo::check_string(&self.region),
            self.subscribers,
            self.mccmnc)
    }
}

pub struct Carriers {
    th_selector: Selector,
    td_selector: Selector,
    h2_selector: Selector,
    tr_selector: Selector,
    re_subs: Regex,
}

impl Carriers {
    pub fn new() -> Self {
        Carriers {
            th_selector: Selector::parse("th").unwrap(),
            td_selector: Selector::parse("td").unwrap(),
            h2_selector: Selector::parse("h2").unwrap(),
            tr_selector: Selector::parse("tr").unwrap(),
            re_subs: Regex::new(r"(\d|\.)+").unwrap(),
        }
    }

    fn parse_header(&self, rows: &ElementRef) -> Result<(usize, usize), ScrapeError> {
        let mut header_iter = rows.select(&self.th_selector).skip(3);

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

    fn parse_carrier(&self, carrier: &ElementRef, country: &str, region: &str, multiplier: usize, mcc: bool) -> Result<CarrierInfo, ScrapeError> {
        // I don't want to deal with any percentage stuff now.
        if multiplier > 0 {
            let mut pointer = carrier
                                .select(&self.td_selector)
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
                        let clean_text = self.re_subs.captures(&subscribers_text)
                            .expect("Parsing number failed")
                            .get(0)
                            .unwrap()
                            .as_str();

                        let subscribers = f64::from_str(clean_text).expect(&format!("Float parsing {} failed for {}", clean_text, operator)) * (multiplier as f64);

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

    async fn parse_page(&self, uri: &str, region: &str) -> Vec<CarrierInfo> {
        info!("Parse page");

        let text = crate::fetch(uri, region).await;

        let mut carriers = Vec::with_capacity(128);

        let fragment = Html::parse_fragment(&text);

        // Not needed right now as moved into using Wikimedia parser API
        //let main_page = Selector::parse(".mw-parser-output").unwrap();
        //let valid_subset = fragment.select(&main_page).next().unwrap();
        let countries = fragment.select(&self.h2_selector);

        let table_selector = Selector::parse("table[class^=wikitable]").unwrap();
        let table = fragment.select(&table_selector);

        // Countries skip jumps over "contents" field in each page
        for (rows, country) in table.zip(countries) {
            let country = country.text().collect::<Vec<_>>()[0].to_string();

            if let Ok((multiplier, count)) = self.parse_header(&rows) {
                for row in rows.select(&self.tr_selector).skip(1) {
                    match self.parse_carrier(&row, &country, &region, multiplier, count > 1) {
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


    pub async fn parse(&self, format: FileFormat, output_path: &mut PathBuf) -> String {
        let world = [
            ("Europe", "https://en.wikipedia.org/w/api.php?action=parse&page=List_of_mobile_network_operators_of_Europe&prop=text&formatversion=2&disabletoc=true&format=json"),
            ("Americas", "https://en.wikipedia.org/w/api.php?action=parse&page=List_of_mobile_network_operators_of_the_Americas&prop=text&formatversion=2&disabletoc=true&format=json"),
            ("MEA", "https://en.wikipedia.org/w/api.php?action=parse&page=List_of_mobile_network_operators_of_the_Middle_East_and_Africa&prop=text&formatversion=2&disabletoc=true&format=json"),
            ("APAC", "https://en.wikipedia.org/w/api.php?action=parse&page=List_of_mobile_network_operators_of_the_Asia_Pacific_region&prop=text&formatversion=2&disabletoc=true&format=json"),
          ];

        let mut all_carriers = Vec::new();

        for (region, uri) in world {
            all_carriers.append(
                &mut self.parse_page(&uri, region).await
            );
        }

        let serialized_carriers = match format {
            FileFormat::JSON => {
                *output_path = output_path.with_extension("json");
                serde_json::to_string(&all_carriers).expect("Serializing carriers failed")
            },
            FileFormat::CSV => {
                *output_path = output_path.with_extension("csv");
                CarrierInfo::gnerate_csv_header() +
                            &all_carriers
                                .iter()
                                .map(|x| x.to_string())
                                .collect::<Vec<String>>()
                                .join("\n")
            }
        };
        serialized_carriers
    }
}