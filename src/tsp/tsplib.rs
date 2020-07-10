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
    pub name: String,
    pub comment: String,
    cities: Vec<KDPoint>,
}

impl TspLibData {
    pub fn new(name: String, comment: String, cities: Vec<KDPoint>) -> Self {
        TspLibData {
            name,
            comment,
            cities,
        }
    }

    pub fn cities(&self) -> &[KDPoint] {
        self.cities.as_ref()
    }

    pub fn len(&self) -> usize {
        self.cities.len()
    }
}

pub fn read_from_file(path: &Path) -> Result<TspLibData, String> {
    let f_res = File::open(path);
    if f_res.is_err() {
        return Err("tsplib: failed to read file".to_owned());
    }

    let reader = BufReader::new(f_res.unwrap());

    process_lines(reader)
}

pub fn read_from_stdin() -> Result<TspLibData, String> {
    let reader = std::io::stdin();

    process_lines(reader.lock())
}

fn process_lines<R: BufRead>(reader: R) -> Result<TspLibData, String> {
    let mut metadata: HashMap<String, String> = HashMap::new();
    let mut cities: Vec<KDPoint> = vec![];

    let mut state = TspReaderStates::START;
    let mut line_no = 1;
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
                    Ok(pt) => cities.push(pt),
                }
            }
            TspReaderStates::END => {
                break;
            }
            _ => continue,
        }
    }

    if cities.is_empty() {
        return Err("Found no valid city coordinates".to_string());
    }

    let unspecified_val = "unspecified".to_string();
    let dt = TspLibData::new(
        metadata
            .get("NAME")
            .unwrap_or(&unspecified_val)
            .to_owned()
            .to_lowercase(),
        metadata
            .get("COMMENT")
            .unwrap_or(&unspecified_val)
            .to_owned()
            .to_lowercase(),
        cities,
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

    #[test]
    fn test_is_state_marker_with_empty_string() {
        let line = "".to_string();

        assert!(!is_state_marker(&line))
    }

    #[test]
    fn test_is_state_marker_with_metadata() {
        let line = "KEY: VALUE".to_string();

        assert!(!is_state_marker(&line));
    }

    #[test]
    fn test_is_state_marker_with_state_key() {
        let line = "EOF".to_string();

        assert!(is_state_marker(&line));
    }

    #[test]
    fn test_next_state_with_end_marker() {
        let state = TspReaderStates::OUTSECTION;

        assert_eq!(TspReaderStates::END, next_state(&state, &"EOF".to_string()))
    }

    #[test]
    fn test_next_state_with_beginning_of_coord_section() {
        let state = TspReaderStates::START;

        let res = next_state(&state, &"NODE_COORD_SECTION".to_string());
        assert_eq!(
            TspReaderStates::INSECTION(COORD_SECTION_KEY.to_string()),
            res
        );
    }

    #[test]
    fn test_state_from_coord_section_switches_to_outsection() {
        let state = TspReaderStates::INSECTION(COORD_SECTION_KEY.to_string());

        let res = next_state(&state, &"KEY: VAL1".to_string());
        assert_eq!(TspReaderStates::OUTSECTION, res);
    }

    #[test]
    fn test_process_lines_happy_case() {
        let cursor =
            "NAME: case1\nCOMMENT: happy case\nNODE_COORD_SECTION\n1 2.0 3.0\nEOF\n".as_bytes();

        let reader = BufReader::new(cursor);

        let res = process_lines(reader);
        assert!(res.is_ok());

        let dt = res.unwrap();
        assert_eq!("case1".to_string(), dt.name);
        assert_eq!("happy case".to_string(), dt.comment);
        assert_eq!(1, dt.len());

        let pt = dt.cities().get(0).unwrap();
        assert_eq!(1, pt.id);
        assert_eq!(Some(2.0), pt.get(0));
        assert_eq!(Some(3.0), pt.get(1));
    }

    #[test]
    fn test_process_lines_with_empty_string() {
        let cursor = "".as_bytes();
        let reader = BufReader::new(cursor);

        assert!(process_lines(reader).is_err());
    }
}
