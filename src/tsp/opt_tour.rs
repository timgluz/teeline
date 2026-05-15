use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::str::FromStr;
use std::sync::LazyLock;

use regex::Regex;

static KEY_VALUE_MATCHER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<key>\w+)\s*:\s*(?P<val>.+)$").unwrap());

#[derive(Debug, Clone)]
pub struct OptTour {
    pub name: String,
    pub comment: String,
    pub dimension: usize,
    pub route: Vec<usize>,
}

#[derive(Debug, PartialEq)]
enum ParseState {
    Header,
    TourSection,
    End,
}

pub fn read_from_file(path: &Path) -> Result<OptTour, String> {
    let f = File::open(path).map_err(|e| format!("opt_tour: cannot open file: {e}"))?;
    parse_from_reader(BufReader::new(f))
}

#[cfg(test)]
pub(crate) fn parse_from_str(text: &str) -> Result<OptTour, String> {
    parse_from_reader(text.as_bytes())
}

fn parse_from_reader<R: BufRead>(reader: R) -> Result<OptTour, String> {
    let mut metadata: HashMap<String, String> = HashMap::new();
    let mut route: Vec<usize> = Vec::new();
    let mut state = ParseState::Header;

    for line_result in reader.lines() {
        let raw = line_result.map_err(|e| format!("opt_tour: read error: {e}"))?;
        let line = raw.trim().to_uppercase();

        if line == "EOF" || state == ParseState::End {
            break;
        }

        match state {
            ParseState::Header => {
                if line == "TOUR_SECTION" {
                    state = ParseState::TourSection;
                } else if let Some(caps) = KEY_VALUE_MATCHER.captures(&line) {
                    metadata.insert(caps["key"].to_string(), caps["val"].trim().to_string());
                }
            }
            ParseState::TourSection => {
                for token in line.split_whitespace() {
                    match isize::from_str(token) {
                        Ok(-1) => {
                            state = ParseState::End;
                            break;
                        }
                        Ok(id) if id > 0 => route.push(id as usize),
                        _ => {}
                    }
                }
            }
            ParseState::End => break,
        }
    }

    let doc_type = metadata.get("TYPE").map(|s| s.as_str()).unwrap_or("");
    if doc_type != "TOUR" {
        return Err(format!(
            "opt_tour: expected TYPE : TOUR, found TYPE : {doc_type}"
        ));
    }

    let dimension: usize = metadata
        .get("DIMENSION")
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);

    if route.len() != dimension {
        return Err(format!(
            "opt_tour: dimension mismatch — DIMENSION={dimension} but parsed {} cities",
            route.len()
        ));
    }

    Ok(OptTour {
        name: metadata
            .get("NAME")
            .cloned()
            .unwrap_or_else(|| "unknown".to_string()),
        comment: metadata.get("COMMENT").cloned().unwrap_or_default(),
        dimension,
        route,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_TOUR: &str = "\
NAME : test3.opt.tour
COMMENT : three-city test
TYPE : TOUR
DIMENSION : 3
TOUR_SECTION
1
2
3
-1
EOF";

    const MULTI_ID_LINE_TOUR: &str = "\
NAME : test4.opt.tour
TYPE : TOUR
DIMENSION : 4
TOUR_SECTION
1 2
3 4
-1
EOF";

    const WRONG_TYPE_TOUR: &str = "\
NAME : bad.tsp
TYPE : TSP
DIMENSION : 3
TOUR_SECTION
1 2 3
-1
EOF";

    const DIMENSION_MISMATCH_TOUR: &str = "\
NAME : bad.opt.tour
TYPE : TOUR
DIMENSION : 5
TOUR_SECTION
1
2
3
-1
EOF";

    #[test]
    fn test_parse_valid_tour() {
        let result = parse_from_str(VALID_TOUR).unwrap();
        assert_eq!(result.name, "TEST3.OPT.TOUR");
        assert_eq!(result.dimension, 3);
        assert_eq!(result.route, vec![1usize, 2, 3]);
    }

    #[test]
    fn test_parse_tour_multiple_ids_per_line() {
        let result = parse_from_str(MULTI_ID_LINE_TOUR).unwrap();
        assert_eq!(result.dimension, 4);
        assert_eq!(result.route, vec![1usize, 2, 3, 4]);
    }

    #[test]
    fn test_parse_returns_error_on_wrong_type() {
        let result = parse_from_str(WRONG_TYPE_TOUR);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("TYPE"), "error should mention TYPE, got: {msg}");
    }

    #[test]
    fn test_parse_returns_error_on_dimension_mismatch() {
        let result = parse_from_str(DIMENSION_MISMATCH_TOUR);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("dimension") || msg.contains("3") || msg.contains("5"),
            "error should mention dimension mismatch, got: {msg}"
        );
    }
}
