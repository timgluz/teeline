use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::str::FromStr;

use std::sync::LazyLock;

use regex::Regex;

use super::distance_matrix::{self, DistanceMatrix};
use super::kdtree::KDPoint;
use super::CityTable;

const COORD_SECTION_KEY: &str = "NODE_COORD_SECTION";
const DISPLAY_DATA_SECTION_KEY: &str = "DISPLAY_DATA_SECTION";
const EDGE_WEIGHT_SECTION_KEY: &str = "EDGE_WEIGHT_SECTION";
const EOF_KEY: &str = "EOF";

static SECTION_START_MATCHER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<key>[A-Z_]\w*)$").unwrap());
static KEY_VALUE_MATCHER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<key>\w+)\s*:\s*(?P<val>.+)$").unwrap());

#[derive(Debug, Clone)]
pub struct TspLibData {
    pub name: String,
    pub comment: String,
    cities: Vec<KDPoint>,
    dimension: usize,
    raw_distances: Option<Vec<f32>>,
}

impl TspLibData {
    pub fn new(
        name: String,
        comment: String,
        cities: Vec<KDPoint>,
        dimension: usize,
        raw_distances: Option<Vec<f32>>,
    ) -> Self {
        TspLibData {
            name,
            comment,
            cities,
            dimension,
            raw_distances,
        }
    }

    pub fn cities(&self) -> &[KDPoint] {
        self.cities.as_ref()
    }

    pub fn len(&self) -> usize {
        self.cities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cities.is_empty()
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    pub fn has_explicit_weights(&self) -> bool {
        self.raw_distances.is_some()
    }

    pub fn raw_distances(&self) -> Option<&[f32]> {
        self.raw_distances.as_deref()
    }

    pub fn distance_matrix(&self) -> Result<DistanceMatrix, String> {
        match &self.raw_distances {
            Some(dists) => {
                let n = self.cities.len();
                let city_table: CityTable = self
                    .cities
                    .iter()
                    .enumerate()
                    .map(|(i, c)| (i, c.clone()))
                    .collect();
                Ok(DistanceMatrix::new(n, dists.clone(), city_table))
            }
            None => Ok(distance_matrix::from_cities(&self.cities)),
        }
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
    let mut raw_weight_tokens: Vec<f32> = vec![];

    let mut state = TspReaderStates::Start;
    let mut line_no = 1;
    for line_res in reader.lines() {
        if line_res.is_err() {
            return Err(format!("Failed to read line.{:?}", line_no));
        }

        let line = line_res.unwrap().trim().to_uppercase();
        line_no += 1;

        if state == TspReaderStates::End {
            break;
        }

        // -- UPDATE STATE
        if is_state_marker(&line) {
            state = next_state(&state, &line);
            continue;
        }

        // -- EXTRACT VALUE
        match &state {
            TspReaderStates::Start => match KEY_VALUE_MATCHER.captures(&line) {
                None => return Err(format!("Failed to extract meta data on line.{:?}", line_no)),
                Some(res) => {
                    metadata.insert(res["key"].to_string(), res["val"].to_string());
                }
            },
            TspReaderStates::Insection(section_id)
                if (section_id == COORD_SECTION_KEY
                    || section_id == DISPLAY_DATA_SECTION_KEY) =>
            {
                match coords_from_text(line_no, &line) {
                    Err(msg) => return Err(msg),
                    Ok(pt) => cities.push(pt),
                }
            }
            TspReaderStates::Insection(section_id)
                if section_id == EDGE_WEIGHT_SECTION_KEY =>
            {
                raw_weight_tokens.extend(
                    line.split_whitespace()
                        .filter_map(|t| f32::from_str(t).ok()),
                );
            }
            TspReaderStates::End => {
                break;
            }
            _ => continue,
        }
    }

    // Reject ATSP
    if metadata.get("TYPE").map(|v| v.trim()) == Some("ATSP") {
        return Err("ATSP (asymmetric TSP) is not supported".to_string());
    }

    let dimension: usize = metadata
        .get("DIMENSION")
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);

    let raw_distances: Option<Vec<f32>> = if raw_weight_tokens.is_empty() {
        None
    } else {
        let fmt = metadata
            .get("EDGE_WEIGHT_FORMAT")
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        let packed = match fmt.as_str() {
            "FULL_MATRIX" => parse_full_matrix(&raw_weight_tokens, dimension)?,
            "UPPER_ROW" => parse_upper_row(&raw_weight_tokens, dimension)?,
            "LOWER_DIAG_ROW" => parse_lower_diag_row(&raw_weight_tokens, dimension)?,
            other => {
                return Err(format!("Unsupported EDGE_WEIGHT_FORMAT: {other}"));
            }
        };
        Some(packed)
    };

    // Generate grid placeholder coords when no coord section was present
    if cities.is_empty() && raw_distances.is_some() {
        cities = grid_coords(dimension);
    }

    if cities.is_empty() && raw_distances.is_none() {
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
        dimension,
        raw_distances,
    );

    Ok(dt)
}

fn grid_coords(n: usize) -> Vec<KDPoint> {
    let cols = (n as f64).sqrt().ceil() as usize;
    (0..n)
        .map(|i| KDPoint::new_with_id(i + 1, &[(i % cols) as f32, (i / cols) as f32]))
        .collect()
}

fn parse_full_matrix(tokens: &[f32], n: usize) -> Result<Vec<f32>, String> {
    if tokens.len() != n * n {
        return Err(format!(
            "FULL_MATRIX: expected {} tokens, got {}",
            n * n,
            tokens.len()
        ));
    }
    Ok((1..n)
        .flat_map(|i| (0..i).map(move |j| tokens[i * n + j]))
        .collect())
}

fn parse_upper_row(tokens: &[f32], n: usize) -> Result<Vec<f32>, String> {
    let expected = n * (n - 1) / 2;
    if tokens.len() != expected {
        return Err(format!(
            "UPPER_ROW: expected {expected} tokens, got {}",
            tokens.len()
        ));
    }
    let mut matrix = vec![0.0f32; n * n];
    let mut idx = 0;
    for i in 0..n - 1 {
        for j in i + 1..n {
            matrix[i * n + j] = tokens[idx];
            matrix[j * n + i] = tokens[idx];
            idx += 1;
        }
    }
    let mut result = Vec::with_capacity(n * (n - 1) / 2);
    for i in 1..n {
        for j in 0..i {
            result.push(matrix[i * n + j]);
        }
    }
    Ok(result)
}

fn parse_lower_diag_row(tokens: &[f32], n: usize) -> Result<Vec<f32>, String> {
    let expected = n * (n + 1) / 2;
    if tokens.len() != expected {
        return Err(format!(
            "LOWER_DIAG_ROW: expected {expected} tokens, got {}",
            tokens.len()
        ));
    }
    let mut result = Vec::with_capacity(n * (n - 1) / 2);
    let mut idx = 0;
    for i in 0..n {
        result.extend_from_slice(&tokens[idx..idx + i]);
        idx += i + 1;
    }
    Ok(result)
}

fn is_state_marker(line: &str) -> bool {
    SECTION_START_MATCHER.captures(line).is_some()
}

fn next_state(state: &TspReaderStates, line: &str) -> TspReaderStates {
    if line == EOF_KEY {
        return TspReaderStates::End;
    }

    if let Some(res) = SECTION_START_MATCHER.captures(line) {
        return TspReaderStates::Insection(res["key"].to_string());
    }

    match &state {
        TspReaderStates::Insection(_) if !starts_with_number(line) => TspReaderStates::Outsection,
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
    Start,
    Insection(String),
    Outsection,
    End,
}

fn coords_from_text<S: AsRef<str>>(line_no: usize, txt: S) -> Result<KDPoint, String> {
    if starts_with_number(txt.as_ref()) {
        let text = String::from_str(txt.as_ref()).unwrap();
        let mut tokens = text.split_whitespace();
        let id_str = tokens.next().unwrap();
        let id: usize = usize::from_str(id_str).unwrap();

        let coords_res: Result<Vec<f32>, _> = tokens.map(f32::from_str).collect();
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

    // ── existing tests ────────────────────────────────────────────────────────

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
        let state = TspReaderStates::Outsection;

        assert_eq!(TspReaderStates::End, next_state(&state, "EOF"))
    }

    #[test]
    fn test_next_state_with_beginning_of_coord_section() {
        let state = TspReaderStates::Start;

        let res = next_state(&state, "NODE_COORD_SECTION");
        assert_eq!(
            TspReaderStates::Insection(COORD_SECTION_KEY.to_string()),
            res
        );
    }

    #[test]
    fn test_state_from_coord_section_switches_to_outsection() {
        let state = TspReaderStates::Insection(COORD_SECTION_KEY.to_string());

        let res = next_state(&state, "KEY: VAL1");
        assert_eq!(TspReaderStates::Outsection, res);
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

    // ── unpacker unit tests ───────────────────────────────────────────────────
    // Canonical 3-city symmetric matrix: d(0,1)=1, d(0,2)=2, d(1,2)=3
    // Expected lower triangle: [d(1,0)=1, d(2,0)=2, d(2,1)=3]

    #[test]
    fn test_unpack_full_matrix() {
        // 3x3 row-major: d(i,j) at [i*3+j]
        let tokens = [0.0, 1.0, 2.0, 1.0, 0.0, 3.0, 2.0, 3.0, 0.0];
        let result = parse_full_matrix(&tokens, 3).unwrap();
        assert_eq!(vec![1.0, 2.0, 3.0], result);
    }

    #[test]
    fn test_unpack_upper_row() {
        // Upper row (excl. diagonal): row0=[d(0,1),d(0,2)], row1=[d(1,2)]
        let tokens = [1.0, 2.0, 3.0];
        let result = parse_upper_row(&tokens, 3).unwrap();
        assert_eq!(vec![1.0, 2.0, 3.0], result);
    }

    #[test]
    fn test_unpack_lower_diag_row() {
        // Lower diag row (incl. diagonal): row0=[0], row1=[d(1,0),0], row2=[d(2,0),d(2,1),0]
        let tokens = [0.0, 1.0, 0.0, 2.0, 3.0, 0.0];
        let result = parse_lower_diag_row(&tokens, 3).unwrap();
        assert_eq!(vec![1.0, 2.0, 3.0], result);
    }

    #[test]
    fn test_unpack_full_matrix_wrong_size() {
        let tokens = [0.0, 1.0, 2.0]; // too short for n=3
        assert!(parse_full_matrix(&tokens, 3).is_err());
    }

    #[test]
    fn test_unpack_upper_row_wrong_size() {
        let tokens = [1.0, 2.0]; // expected 3 for n=3
        assert!(parse_upper_row(&tokens, 3).is_err());
    }

    #[test]
    fn test_unpack_lower_diag_row_wrong_size() {
        let tokens = [0.0, 1.0]; // expected 6 for n=3
        assert!(parse_lower_diag_row(&tokens, 3).is_err());
    }

    // ── grid_coords ───────────────────────────────────────────────────────────

    #[test]
    fn test_grid_coords_layout() {
        let pts = grid_coords(4);
        // 4 cities in a 2×2 grid; IDs are 1-based
        assert_eq!(4, pts.len());
        assert_eq!(1, pts[0].id);
        assert_eq!(Some(0.0), pts[0].get(0)); // col
        assert_eq!(Some(0.0), pts[0].get(1)); // row
        assert_eq!(2, pts[1].id);
        assert_eq!(Some(1.0), pts[1].get(0));
        assert_eq!(Some(0.0), pts[1].get(1));
        assert_eq!(3, pts[2].id);
        assert_eq!(Some(0.0), pts[2].get(0));
        assert_eq!(Some(1.0), pts[2].get(1));
        assert_eq!(4, pts[3].id);
        assert_eq!(Some(1.0), pts[3].get(0));
        assert_eq!(Some(1.0), pts[3].get(1));
    }

    // ── process_lines with explicit matrix ───────────────────────────────────

    #[test]
    fn test_process_lines_full_matrix() {
        let cursor = b"NAME: test\nDIMENSION: 3\nEDGE_WEIGHT_TYPE: EXPLICIT\nEDGE_WEIGHT_FORMAT: FULL_MATRIX\nEDGE_WEIGHT_SECTION\n0 1 2\n1 0 3\n2 3 0\nEOF\n";
        let reader = BufReader::new(cursor.as_ref());
        let dt = process_lines(reader).unwrap();
        assert!(dt.has_explicit_weights());
        assert_eq!(3, dt.dimension());
        let dm = dt.distance_matrix().unwrap();
        assert!((dm.distance_between(1, 2).unwrap() - 1.0).abs() < 1e-3);
        assert!((dm.distance_between(1, 3).unwrap() - 2.0).abs() < 1e-3);
        assert!((dm.distance_between(2, 3).unwrap() - 3.0).abs() < 1e-3);
    }

    #[test]
    fn test_process_lines_upper_row() {
        let cursor = b"NAME: test\nDIMENSION: 3\nEDGE_WEIGHT_TYPE: EXPLICIT\nEDGE_WEIGHT_FORMAT: UPPER_ROW\nEDGE_WEIGHT_SECTION\n1 2 3\nEOF\n";
        let reader = BufReader::new(cursor.as_ref());
        let dt = process_lines(reader).unwrap();
        assert!(dt.has_explicit_weights());
        let dm = dt.distance_matrix().unwrap();
        assert!((dm.distance_between(1, 2).unwrap() - 1.0).abs() < 1e-3);
        assert!((dm.distance_between(2, 3).unwrap() - 3.0).abs() < 1e-3);
    }

    #[test]
    fn test_process_lines_lower_diag_row() {
        // row0=[0], row1=[1,0], row2=[2,3,0]
        let cursor = b"NAME: test\nDIMENSION: 3\nEDGE_WEIGHT_TYPE: EXPLICIT\nEDGE_WEIGHT_FORMAT: LOWER_DIAG_ROW\nEDGE_WEIGHT_SECTION\n0 1 0 2 3 0\nEOF\n";
        let reader = BufReader::new(cursor.as_ref());
        let dt = process_lines(reader).unwrap();
        assert!(dt.has_explicit_weights());
        let dm = dt.distance_matrix().unwrap();
        assert!((dm.distance_between(1, 2).unwrap() - 1.0).abs() < 1e-3);
        assert!((dm.distance_between(1, 3).unwrap() - 2.0).abs() < 1e-3);
        assert!((dm.distance_between(2, 3).unwrap() - 3.0).abs() < 1e-3);
    }

    #[test]
    fn test_reject_atsp() {
        let cursor = b"NAME: atsp\nTYPE: ATSP\nDIMENSION: 3\nEDGE_WEIGHT_TYPE: EXPLICIT\nEDGE_WEIGHT_FORMAT: FULL_MATRIX\nEDGE_WEIGHT_SECTION\n0 1 2\n3 0 4\n5 6 0\nEOF\n";
        let reader = BufReader::new(cursor.as_ref());
        let result = process_lines(reader);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ATSP"));
    }

    #[test]
    fn test_explicit_with_display_data() {
        // File has EDGE_WEIGHT_SECTION and DISPLAY_DATA_SECTION (like bayg29)
        let cursor = b"NAME: test\nDIMENSION: 3\nEDGE_WEIGHT_TYPE: EXPLICIT\nEDGE_WEIGHT_FORMAT: UPPER_ROW\nDISPLAY_DATA_TYPE: TWOD_DISPLAY\nEDGE_WEIGHT_SECTION\n1 2 3\nDISPLAY_DATA_SECTION\n1 10.0 20.0\n2 30.0 40.0\n3 50.0 60.0\nEOF\n";
        let reader = BufReader::new(cursor.as_ref());
        let dt = process_lines(reader).unwrap();
        assert!(dt.has_explicit_weights());
        // Cities come from DISPLAY_DATA_SECTION, not grid
        assert_eq!(3, dt.len());
        assert_eq!(Some(10.0), dt.cities()[0].get(0));
    }
}
