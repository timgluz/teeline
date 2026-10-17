use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::str::FromStr;

use std::sync::LazyLock;

use regex::Regex;

use super::CityTable;
use super::DistanceType;
use super::distance_matrix::{self, DistanceMatrix};
use super::kdtree::KDPoint;

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
    pub distance_type: DistanceType,
}

impl TspLibData {
    pub fn new(
        name: String,
        comment: String,
        cities: Vec<KDPoint>,
        dimension: usize,
        raw_distances: Option<Vec<f32>>,
        distance_type: DistanceType,
    ) -> Self {
        TspLibData {
            name,
            comment,
            cities,
            dimension,
            raw_distances,
            distance_type,
        }
    }

    pub fn with_distance_type(mut self, dt: DistanceType) -> Self {
        self.distance_type = dt;
        self
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
                    .map(|(i, c)| (i, *c))
                    .collect();
                Ok(DistanceMatrix::new(n, dists.clone(), city_table))
            }
            None => Ok(distance_matrix::build(&self.cities, self.distance_type)),
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

pub fn read_from_str(input: &str) -> Result<TspLibData, String> {
    let cursor = std::io::Cursor::new(input.as_bytes());
    process_lines(BufReader::new(cursor))
}

#[cfg(feature = "json")]
pub fn parse_json_cities(input: &str) -> Result<Vec<super::KDPoint>, String> {
    #[derive(serde::Deserialize)]
    struct CityRecord {
        id: usize,
        x: f32,
        y: f32,
    }
    let records: Vec<CityRecord> =
        serde_json::from_str(input).map_err(|e| format!("JSON parse error: {e}"))?;
    if records.len() < 2 {
        return Err("need at least 2 cities".into());
    }
    Ok(records
        .into_iter()
        .map(|c| super::KDPoint::new_with_id(c.id, &[c.x, c.y]))
        .collect())
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
                if (section_id == COORD_SECTION_KEY || section_id == DISPLAY_DATA_SECTION_KEY) =>
            {
                let pt = coords_from_text(line_no, &line)?;
                cities.push(pt)
            }
            TspReaderStates::Insection(section_id) if section_id == EDGE_WEIGHT_SECTION_KEY => {
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

    // An absent key defaults to EUC_2D (nothing was declared); a *declared* but unsupported
    // type is an error. The previous `.ok().unwrap_or_default()` silently measured MAN_2D,
    // MAX_2D, EUC_3D and friends as planar Euclidean — the same silent-wrong-answer class
    // the ATT/CEIL_2D support above exists to remove.
    let distance_type: DistanceType = match metadata.get("EDGE_WEIGHT_TYPE") {
        None => DistanceType::default(),
        Some(v) => v
            .trim()
            .parse::<DistanceType>()
            .map_err(|e| e.to_string())?,
    };

    let dimension: usize = metadata
        .get("DIMENSION")
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);

    let raw_distances: Option<Vec<f32>> = if raw_weight_tokens.is_empty() {
        None
    } else {
        let fmt = metadata
            .get("EDGE_WEIGHT_FORMAT")
            .map(|v| v.trim().to_uppercase())
            .unwrap_or_default();
        let packed = parse_explicit(&raw_weight_tokens, dimension, &fmt)?;
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
        distance_type,
    );

    Ok(dt)
}

fn grid_coords(n: usize) -> Vec<KDPoint> {
    let cols = (n as f64).sqrt().ceil() as usize;
    (0..n)
        .map(|i| KDPoint::new_with_id(i + 1, &[(i % cols) as f32, (i / cols) as f32]))
        .collect()
}

/// The entries an `EDGE_WEIGHT_FORMAT` writes, in the order the file stores them. TSPLIB has nine
/// layouts: each picks a triangle, includes the diagonal or not, and walks it row-wise or
/// column-wise. They are all the same matrix, so a reader that transposes one of them produces a
/// plausible wrong answer rather than an error — which is why they share one reader.
fn explicit_entries(format: &str, n: usize) -> Result<Vec<(usize, usize)>, String> {
    Ok(match format {
        "FULL_MATRIX" => (0..n).flat_map(|i| (0..n).map(move |j| (i, j))).collect(),
        "UPPER_ROW" => (0..n)
            .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
            .collect(),
        "LOWER_ROW" => (0..n).flat_map(|i| (0..i).map(move |j| (i, j))).collect(),
        "UPPER_DIAG_ROW" => (0..n).flat_map(|i| (i..n).map(move |j| (i, j))).collect(),
        "LOWER_DIAG_ROW" => (0..n).flat_map(|i| (0..=i).map(move |j| (i, j))).collect(),
        "UPPER_COL" => (0..n).flat_map(|j| (0..j).map(move |i| (i, j))).collect(),
        "LOWER_COL" => (0..n)
            .flat_map(|j| (j + 1..n).map(move |i| (i, j)))
            .collect(),
        "UPPER_DIAG_COL" => (0..n).flat_map(|j| (0..=j).map(move |i| (i, j))).collect(),
        "LOWER_DIAG_COL" => (0..n).flat_map(|j| (j..n).map(move |i| (i, j))).collect(),
        "" => return Err("EDGE_WEIGHT_SECTION without an EDGE_WEIGHT_FORMAT".to_string()),
        other => return Err(format!("Unsupported EDGE_WEIGHT_FORMAT: {other}")),
    })
}

/// Decode an `EDGE_WEIGHT_SECTION` into the packed lower triangle, row-wise — the one shape
/// `DistanceMatrix` reads, whatever layout the file used.
///
/// The matrix is reconstructed in full from the triangle the format stores, because TSPLIB's
/// `TYPE: TSP` means the matrix is symmetric; `TYPE: ATSP` never reaches here (it is rejected),
/// so mirroring the stored triangle is the whole of the asymmetry question.
fn parse_explicit(tokens: &[f32], n: usize, format: &str) -> Result<Vec<f32>, String> {
    let entries = explicit_entries(format, n)?;
    if tokens.len() != entries.len() {
        return Err(format!(
            "{format}: expected {} tokens, got {}",
            entries.len(),
            tokens.len()
        ));
    }

    let mut matrix = vec![0.0f32; n * n];
    for (token, (i, j)) in tokens.iter().zip(entries) {
        matrix[i * n + j] = *token;
        matrix[j * n + i] = *token;
    }

    let mut packed = Vec::with_capacity(n * (n - 1) / 2);
    for i in 1..n {
        for j in 0..i {
            packed.push(matrix[i * n + j]);
        }
    }
    Ok(packed)
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
        // KDPoint is 2-D only; extra coordinates in the input are silently ignored.
        let txt = "3 1.0 -2.0 3";

        let res = coords_from_text(0, txt);
        assert!(res.is_ok());

        let pt = res.unwrap();
        assert_eq!(3, pt.id);
        assert_eq!(2, pt.dim());
        assert_eq!(Some(1.0), pt.get(0));
        assert_eq!(Some(-2.0), pt.get(1));
        assert_eq!(None, pt.get(2));
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
    // Expected lower triangle, row-wise: [d(1,0)=1, d(2,0)=2, d(2,1)=3]

    /// The nine layouts, each holding that same 3-city matrix. n=3 needs 3 tokens without the
    /// diagonal and 6 with it, so the cases double as a length check for every format.
    const THREE_CITY_LAYOUTS: [(&str, &[f32]); 9] = [
        // row-major 3x3
        (
            "FULL_MATRIX",
            &[0.0, 1.0, 2.0, 1.0, 0.0, 3.0, 2.0, 3.0, 0.0],
        ),
        ("UPPER_ROW", &[1.0, 2.0, 3.0]),
        ("LOWER_ROW", &[1.0, 2.0, 3.0]),
        ("UPPER_DIAG_ROW", &[0.0, 1.0, 2.0, 0.0, 3.0, 0.0]),
        ("LOWER_DIAG_ROW", &[0.0, 1.0, 0.0, 2.0, 3.0, 0.0]),
        ("UPPER_COL", &[1.0, 2.0, 3.0]),
        ("LOWER_COL", &[1.0, 2.0, 3.0]),
        ("UPPER_DIAG_COL", &[0.0, 1.0, 0.0, 2.0, 3.0, 0.0]),
        ("LOWER_DIAG_COL", &[0.0, 1.0, 2.0, 0.0, 3.0, 0.0]),
    ];

    #[test]
    fn test_unpack_every_edge_weight_format() {
        for (format, tokens) in THREE_CITY_LAYOUTS {
            let result =
                parse_explicit(tokens, 3, format).unwrap_or_else(|e| panic!("{format}: {e}"));
            assert_eq!(
                vec![1.0, 2.0, 3.0],
                result,
                "{format} should pack the same matrix as every other layout"
            );
        }
    }

    #[test]
    fn test_unpack_keeps_the_row_and_column_orders_distinct() {
        // The 3-city cases above cannot tell a row-wise read from a column-wise one: at n=3 both
        // orders visit the same single pair per row/column, so every layout takes the same token
        // sequence. At n=5 they differ — UPPER_ROW reads d(1,4) third, UPPER_COL reads d(2,3)
        // there — so decoding one order's tokens with the other's mapping must change the matrix.
        // This is the failure the shared reader exists to prevent: a transposed estimate.
        let tokens = [1.0, 2.0, 5.0, 3.0, 6.0, 8.0, 4.0, 7.0, 9.0, 10.0]; // d12; d13,d23; d14,d24,d34; d15,…
        let as_upper_col = parse_explicit(&tokens, 5, "UPPER_COL").unwrap();
        let as_upper_row = parse_explicit(&tokens, 5, "UPPER_ROW").unwrap();
        assert_eq!(
            vec![1.0, 2.0, 5.0, 3.0, 6.0, 8.0, 4.0, 7.0, 9.0, 10.0],
            as_upper_col,
            "UPPER_COL tokens should decode to the matrix they describe"
        );
        assert_ne!(
            as_upper_col, as_upper_row,
            "UPPER_ROW and UPPER_COL must not share one mapping"
        );
    }

    #[test]
    fn test_unpack_rejects_a_length_that_does_not_match_the_format() {
        // 5 tokens is not a valid count for n=3 under any layout. Reading it anyway would shift
        // every distance by one from the first diagonal it assumed.
        for (format, _) in THREE_CITY_LAYOUTS {
            let tokens = [0.0f32; 5];
            assert!(
                parse_explicit(&tokens, 3, format).is_err(),
                "{format} accepted 5 tokens for a 3-city matrix"
            );
        }
    }

    #[test]
    fn test_unpack_rejects_an_unknown_format() {
        let err = parse_explicit(&[1.0, 2.0, 3.0], 3, "UPPER_TRIANGLE").unwrap_err();
        assert!(
            err.contains("UPPER_TRIANGLE"),
            "error should name it: {err}"
        );
    }

    #[test]
    fn test_unpack_reports_a_missing_format_as_missing() {
        // The message has to distinguish "this file declares nothing" from "we do not know this
        // one", because only the second is our gap.
        let err = parse_explicit(&[1.0, 2.0, 3.0], 3, "").unwrap_err();
        assert!(err.contains("without an EDGE_WEIGHT_FORMAT"), "{err}");
        assert!(!err.contains("Unsupported"), "{err}");
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
    fn test_process_lines_parses_every_edge_weight_format() {
        // Five cities, d(1,2)=1 … d(4,5)=10. Reading through process_lines covers the dispatch as
        // well as the layout, which the unpacker tests above cannot.
        let layouts = [
            (
                "FULL_MATRIX",
                "0 1 2 3 4 1 0 5 6 7 2 5 0 8 9 3 6 8 0 10 4 7 9 10 0",
            ),
            ("UPPER_ROW", "1 2 3 4 5 6 7 8 9 10"),
            ("LOWER_ROW", "1 2 5 3 6 8 4 7 9 10"),
            ("UPPER_DIAG_ROW", "0 1 2 3 4 0 5 6 7 0 8 9 0 10 0"),
            ("LOWER_DIAG_ROW", "0 1 0 2 5 0 3 6 8 0 4 7 9 10 0"),
            ("UPPER_COL", "1 2 5 3 6 8 4 7 9 10"),
            ("LOWER_COL", "1 2 3 4 5 6 7 8 9 10"),
            ("UPPER_DIAG_COL", "0 1 0 2 5 0 3 6 8 0 4 7 9 10 0"),
            ("LOWER_DIAG_COL", "0 1 2 3 4 0 5 6 7 0 8 9 0 10 0"),
        ];
        let expected = [
            (1, 2, 1.0),
            (1, 3, 2.0),
            (1, 4, 3.0),
            (1, 5, 4.0),
            (2, 3, 5.0),
            (2, 4, 6.0),
            (2, 5, 7.0),
            (3, 4, 8.0),
            (3, 5, 9.0),
            (4, 5, 10.0),
        ];

        for (format, weights) in layouts {
            let cursor = format!(
                "NAME: t\nDIMENSION: 5\nEDGE_WEIGHT_TYPE: EXPLICIT\nEDGE_WEIGHT_FORMAT: {format}\nEDGE_WEIGHT_SECTION\n{weights}\nEOF\n"
            );
            let dt = process_lines(BufReader::new(cursor.as_bytes()))
                .unwrap_or_else(|e| panic!("{format}: {e}"));
            assert!(dt.has_explicit_weights(), "{format}");
            let dm = dt.distance_matrix().unwrap();
            for (a, b, want) in expected {
                let got = dm.distance_between(a, b).unwrap();
                assert!(
                    (got - want).abs() < 1e-3,
                    "{format}: d({a},{b}) = {got}, want {want}"
                );
            }
        }
    }

    #[test]
    fn test_process_lines_rejects_a_matrix_without_a_format() {
        // Without EDGE_WEIGHT_FORMAT there is no way to know how the section is laid out, and
        // guessing one would silently mis-measure the instance.
        let cursor =
            "NAME: t\nDIMENSION: 3\nEDGE_WEIGHT_TYPE: EXPLICIT\nEDGE_WEIGHT_SECTION\n1 2 3\nEOF\n";
        let err = process_lines(BufReader::new(cursor.as_bytes())).unwrap_err();
        assert!(err.contains("without an EDGE_WEIGHT_FORMAT"), "{err}");
    }

    #[test]
    fn test_process_lines_geo_distance_type() {
        let cursor = "NAME: geo_case\nEDGE_WEIGHT_TYPE: GEO\nNODE_COORD_SECTION\n1 16.47 96.10\n2 23.70 96.99\nEOF\n".as_bytes();
        let reader = BufReader::new(cursor);
        let res = process_lines(reader);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().distance_type, DistanceType::Geo);
    }

    #[test]
    fn test_process_lines_rejects_an_unsupported_declared_type() {
        // This test previously asserted that an unrecognised type silently became EUC_2D,
        // using ATT as its example — which is how the ATT mis-measurement survived: the
        // behaviour was asserted as intended. A declared type we cannot honour is now an
        // error, because measuring it as planar Euclidean yields plausible wrong answers.
        let cursor = "NAME: bogus_case\nEDGE_WEIGHT_TYPE: MAN_2D\nNODE_COORD_SECTION\n1 2.0 3.0\n2 4.0 5.0\nEOF\n".as_bytes();
        let res = process_lines(BufReader::new(cursor));
        assert!(
            res.is_err(),
            "MAN_2D must not be silently measured as EUC_2D"
        );
        assert!(
            res.unwrap_err().contains("MAN_2D"),
            "the error should name the unsupported type"
        );
    }

    #[test]
    fn test_process_lines_defaults_to_euc2d_when_type_absent() {
        // Absent is not the same as unsupported: a missing key still defaults.
        let cursor = "NAME: untyped\nNODE_COORD_SECTION\n1 2.0 3.0\n2 4.0 5.0\nEOF\n".as_bytes();
        let res = process_lines(BufReader::new(cursor));
        assert!(
            res.is_ok(),
            "an absent EDGE_WEIGHT_TYPE should still default"
        );
        assert_eq!(res.unwrap().distance_type, DistanceType::Euc2D);
    }

    #[test]
    fn test_process_lines_parses_att_and_ceil_2d() {
        for (declared, expected) in [
            ("ATT", DistanceType::Att),
            ("CEIL_2D", DistanceType::Ceil2D),
        ] {
            let cursor = format!(
                "NAME: t\nEDGE_WEIGHT_TYPE: {declared}\nNODE_COORD_SECTION\n1 2.0 3.0\n2 4.0 5.0\nEOF\n"
            );
            let res = process_lines(BufReader::new(cursor.as_bytes()));
            assert!(res.is_ok(), "{declared} should parse");
            assert_eq!(res.unwrap().distance_type, expected);
        }
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

    // ── parse_json_cities tests ───────────────────────────────────────────────

    #[cfg(feature = "json")]
    #[test]
    fn test_parse_json_cities_valid() {
        let json =
            r#"[{"id":0,"x":1.0,"y":2.0},{"id":1,"x":3.0,"y":4.0},{"id":2,"x":5.0,"y":6.0}]"#;
        let cities = parse_json_cities(json).unwrap();
        assert_eq!(3, cities.len());
        assert_eq!(0, cities[0].id);
        assert_eq!(Some(1.0), cities[0].get(0));
        assert_eq!(2, cities[2].id);
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_parse_json_cities_one_city() {
        let json = r#"[{"id":0,"x":1.0,"y":2.0}]"#;
        let err = parse_json_cities(json).unwrap_err();
        assert!(err.contains("2"), "expected '2' in error, got: {err}");
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_parse_json_cities_empty_array() {
        let err = parse_json_cities("[]").unwrap_err();
        assert!(!err.is_empty());
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_parse_json_cities_bad_type() {
        let json = r#"[{"id":0,"x":"nope","y":1.0}]"#;
        assert!(parse_json_cities(json).is_err());
    }

    // ── read_from_str tests ───────────────────────────────────────────────────

    #[test]
    fn test_read_from_str_minimal() {
        let input = "NAME: mini\nNODE_COORD_SECTION\n1 1.0 2.0\n2 3.0 4.0\nEOF\n";
        let dt = read_from_str(input).unwrap();
        assert_eq!(2, dt.len());
        assert_eq!(1, dt.cities()[0].id);
        assert_eq!(2, dt.cities()[1].id);
    }

    #[test]
    fn test_read_from_str_empty() {
        assert!(read_from_str("").is_err());
    }

    #[test]
    fn test_read_from_str_burma14() {
        let input =
            std::fs::read_to_string("tests/fixtures/burma14.tsp").expect("burma14.tsp missing");
        let dt = read_from_str(&input).unwrap();
        assert_eq!(14, dt.len());
        assert_eq!("burma14", dt.name);
    }

    #[test]
    fn test_read_from_str_berlin52() {
        let input =
            std::fs::read_to_string("tests/fixtures/berlin52.tsp").expect("berlin52.tsp missing");
        let dt = read_from_str(&input).unwrap();
        assert_eq!(52, dt.len());
    }

    #[test]
    fn test_read_from_str_atsp_rejected() {
        let input = "NAME: a\nTYPE: ATSP\nDIMENSION: 2\nEDGE_WEIGHT_TYPE: EXPLICIT\nEDGE_WEIGHT_FORMAT: FULL_MATRIX\nEDGE_WEIGHT_SECTION\n0 1\n1 0\nEOF\n";
        let err = read_from_str(input).unwrap_err();
        assert!(err.contains("ATSP"), "expected ATSP in error, got: {err}");
    }
}
