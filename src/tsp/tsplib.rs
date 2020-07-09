use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::str::FromStr;

use lazy_static::lazy_static;
use regex::Regex;

use super::kdtree::KDPoint;

const COORD_SECTION_KEY: &'static str = "NODE_COORD_SECTION";
const DISPLAY_DATA_SECTION_KEY: &'static str = "DISPLAY_DATA_SECTION";
const EOF_KEY: &'static str = "EOF";

lazy_static! {
    static ref SECTION_START_MATCHER: Regex = Regex::new(r"^(?P<key>\w+)$").unwrap();
    // it matches key value pairs separated by colon(:)
    static ref KEY_VALUE_MATCHER: Regex = Regex::new(r"^(?P<key>\w+)\s*:\s*(?P<val>.+)$").unwrap();
}

#[derive(Debug, Clone)]
pub struct TspLibData {
    name: String,
    comment: String,
    nodes: Vec<KDPoint>,
}

impl TspLibData {
    pub fn new(name: String, comment: String, nodes: Vec<KDPoint>) -> Self {
        TspLibData {
            name,
            comment,
            nodes,
        }
    }

    pub fn nodes(&self) -> &[KDPoint] {
        self.nodes.as_ref()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

pub fn read_from_file(path: &Path) -> Result<TspLibData, String> {
    let f_res = File::open(path);
    if f_res.is_err() {
        return Err("tsplib: failed to read file".to_owned());
    }

    let reader = BufReader::new(f_res.unwrap());

    let mut metadata: HashMap<String, String> = HashMap::new();
    let mut nodes: Vec<KDPoint> = vec![];

    let mut state = TspReaderStates::START;
    let mut line_no = 0;
    for line_res in reader.lines() {
        if line_res.is_err() {
            return Err(format!("Failed to read line.{:?}", line_no));
        }

        let line = line_res.unwrap().trim().to_uppercase();
        line_no += 1;

        if state == TspReaderStates::END {
            break;
        }

        // -- UPDATE STATE
        if is_state_marker(&line) {
            state = next_state(&state, &line);
            continue;
        }

        // -- EXTRACT VALUE
        match &state {
            TspReaderStates::START => match KEY_VALUE_MATCHER.captures(&line) {
                None => return Err(format!("Failed to extract meta data on line.{:?}", line_no)),
                Some(res) => {
                    metadata.insert(res["key"].to_string(), res["val"].to_string());
                }
            },
            // we parse coords only from those 2 sections
            TspReaderStates::INSECTION(section_id)
                if (section_id == COORD_SECTION_KEY || section_id == DISPLAY_DATA_SECTION_KEY) =>
            {
                match coords_from_text(line_no, &line) {
                    Err(msg) => return Err(msg),
                    Ok(pt) => nodes.push(pt),
                }
            }
            TspReaderStates::END => {
                break;
            }
            _ => continue,
        }
    }

    let unspecified_val = "unspecified".to_string();
    let dt = TspLibData::new(
        metadata.get("NAME").unwrap_or(&unspecified_val).to_owned(),
        metadata
            .get("COMMENT")
            .unwrap_or(&unspecified_val)
            .to_owned(),
        nodes,
    );

    Ok(dt)
}

fn is_state_marker(line: &String) -> bool {
    SECTION_START_MATCHER.captures(line).is_some()
}

fn next_state(state: &TspReaderStates, line: &String) -> TspReaderStates {
    if line == EOF_KEY {
        return TspReaderStates::END;
    }

    // if it section keyword
    if let Some(res) = SECTION_START_MATCHER.captures(&line) {
        let next_state = TspReaderStates::INSECTION(res["key"].to_string());

        return next_state;
    }

    // check if we are already out of coord section
    match &state {
        TspReaderStates::INSECTION(_) if !starts_with_number(&line) => TspReaderStates::OUTSECTION,
        st => st.to_owned().clone(),
    }
}

fn starts_with_number<S: AsRef<str>>(txt: S) -> bool {
    let text = String::from_str(txt.as_ref()).unwrap();
    if let Some(first_token) = text.split_whitespace().next() {
        f32::from_str(first_token).is_ok()
    } else {
        false
    }
}

#[derive(Debug, Clone, PartialEq)]
enum TspReaderStates {
    START,
    INSECTION(String),
    OUTSECTION,
    END,
}

fn coords_from_text<S: AsRef<str>>(line_no: usize, txt: S) -> Result<KDPoint, String> {
    if starts_with_number(txt.as_ref()) {
        let text = String::from_str(txt.as_ref()).unwrap();
        let mut tokens = text.split_whitespace();
        let id_str = tokens.next().unwrap().clone();
        let id: usize = usize::from_str(id_str).unwrap();

        // it is important we take id first out, then we dont need skip(1) here
        let coords_res: Result<Vec<f32>, _> = tokens.map(|x| f32::from_str(x)).collect();
        if coords_res.is_err() {
            return Err(format!("Error on line.{:?} - invalid number", line_no));
        }

        let pt = KDPoint::new_with_id(id, &coords_res.unwrap());

        Ok(pt)
    } else {
        Err(format!(
            "Failed to extract coordinates on line.{:?}",
            line_no
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coords_from_text_only_ints() {
        let txt = "1 2 3";

        let res = coords_from_text(0, txt);
        assert!(res.is_ok());
        let pt = res.unwrap();
        assert_eq!(1, pt.id);
        assert_eq!(2, pt.dim());
        assert_eq!(Some(2.0), pt.get(0));
        assert_eq!(Some(3.0), pt.get(1));
    }

    #[test]
    fn test_coords_from_text_include_negative_coord() {
        let txt = "2 1 -2";

        let res = coords_from_text(0, txt);
        assert!(res.is_ok());
        let pt = res.unwrap();
        assert_eq!(2, pt.id);
        assert_eq!(2, pt.dim());
        assert_eq!(Some(1.0), pt.get(0));
        assert_eq!(Some(-2.0), pt.get(1));
    }

    #[test]
    fn test_coords_from_text_3dim_coord() {
        let txt = "3 1.0 -2.0 3";

        let res = coords_from_text(0, txt);
        assert!(res.is_ok());

        let pt = res.unwrap();
        assert_eq!(3, pt.id);
        assert_eq!(3, pt.dim());
        assert_eq!(Some(1.0), pt.get(0));
        assert_eq!(Some(-2.0), pt.get(1));
        assert_eq!(Some(3.0), pt.get(2));
    }
}
