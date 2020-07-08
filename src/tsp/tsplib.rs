use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::str::FromStr;

use regex::Regex;

use super::kdtree::KDPoint;

const COORD_SECTION_KEY: &'static str = "NODE_COORD_SECTION";
const EOF_KEY: &'static str = "EOF";

#[derive(Debug, Clone)]
pub struct TspLib {
    name: String,
    comment: String,
    dimension: usize,
    nodes: Vec<KDPoint>,
}

impl TspLib {
    pub fn new(name: String, comment: String, dimension: usize, nodes: Vec<KDPoint>) -> Self {
        TspLib {
            name,
            comment,
            dimension,
            nodes,
        }
    }

    pub fn nodes(&self) -> &[KDPoint] {
        self.nodes.as_ref()
    }
}

pub fn read_from_file(path: &Path) -> Result<usize, String> {
    let f_res = File::open(path);
    if f_res.is_err() {
        return Err("tsplib: failed to read file".to_owned());
    }

    let mut reader = BufReader::new(f_res.unwrap());

    let meta_data_matcher = Regex::new(r"^(?P<key>\w+)\s*:\s*(?P<val>.+)$").unwrap();
    let coord_matcher =
        Regex::new(r"^(?P<id>\d{1,})\s+(?P<x>\d+\.?\d*)\s+(?P<y>\d+\.?\d*)$").unwrap();
    let mut metadata: HashMap<String, String> = HashMap::new();
    let mut state = TspReaderStates::START;

    for line_res in reader.lines() {
        if line_res.is_err() {
            break;
        }

        let line = line_res.unwrap().trim().to_uppercase();
        println!("got line: {:?}", line);

        // manage reader state
        match line.as_str() {
            COORD_SECTION_KEY => {
                state = TspReaderStates::INCOORD;
                continue; // goto next line
            }
            EOF_KEY => state = TspReaderStates::END,
            _ => {}
        };

        if state == TspReaderStates::INCOORD && !starts_with_number(&line) {
            println!("Switching to OUTCOORD");
            state = TspReaderStates::OUTCOORD; // TODO: double check that EOF is end of section and not file
        }

        match state {
            TspReaderStates::START => {
                let res = meta_data_matcher
                    .captures(&line)
                    .expect("Failed to read metadata");
                println!("metadata: {:?}", res);
            }
            TspReaderStates::INCOORD => {
                let res = coord_matcher.captures(&line).expect("Failed to read coord");
                println!("coords: {:?}", res);
            }
            TspReaderStates::END => {
                break;
            }
            _ => continue,
        }
    }

    Ok(0)
}

fn starts_with_number(text: &String) -> bool {
    if let Some(first_token) = text.split_whitespace().next() {
        f32::from_str(first_token).is_ok()
    } else {
        false
    }
}

#[derive(Debug, PartialEq)]
enum TspReaderStates {
    START,
    INCOORD,
    OUTCOORD,
    END,
}
