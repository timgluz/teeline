use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Parse DiscOpt text: skip the first line (city count), read `x y` coordinate pairs.
pub fn parse_discopt(input: &str) -> Result<Vec<(f32, f32)>, String> {
    let mut coords = Vec::new();
    for line in input.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let x = parts
            .next()
            .ok_or("missing x")?
            .parse::<f32>()
            .map_err(|e| e.to_string())?;
        let y = parts
            .next()
            .ok_or("missing y")?
            .parse::<f32>()
            .map_err(|e| e.to_string())?;
        coords.push((x, y));
    }
    if coords.is_empty() {
        return Err("no coordinates found".into());
    }
    Ok(coords)
}

/// Write TSPLIB EUC_2D format to `writer`. `name` populates NAME and COMMENT headers.
pub fn write_tsplib(name: &str, coords: &[(f32, f32)], writer: &mut impl Write) -> io::Result<()> {
    writeln!(writer, "NAME: {name}")?;
    writeln!(writer, "TYPE: TSP")?;
    writeln!(writer, "COMMENT: converted from DiscOpt dataset {name}")?;
    writeln!(writer, "DIMENSION: {}", coords.len())?;
    writeln!(writer, "EDGE_WEIGHT_TYPE: EUC_2D")?;
    writeln!(writer, "NODE_COORD_SECTION")?;
    for (i, (x, y)) in coords.iter().enumerate() {
        writeln!(writer, "\t{} {} {}", i + 1, x, y)?;
    }
    writeln!(writer, "EOF")?;
    Ok(())
}

/// Convert a single DiscOpt file to a TSPLIB `.tsp` file.
/// The name used in headers is derived from the input file stem.
pub fn convert_file(input: &Path, output: &Path) -> Result<(), String> {
    let text = fs::read_to_string(input).map_err(|e| e.to_string())?;
    let coords = parse_discopt(&text)?;
    let name = input
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut f = fs::File::create(output).map_err(|e| e.to_string())?;
    write_tsplib(&name, &coords, &mut f).map_err(|e| e.to_string())
}

/// Convert every file in `input_dir` to a TSPLIB `.tsp` file in `output_dir`.
/// Returns `(success_count, Vec<(path, error_message)>)`.
pub fn convert_dir(input_dir: &Path, output_dir: &Path) -> (usize, Vec<(PathBuf, String)>) {
    let entries = match fs::read_dir(input_dir) {
        Ok(e) => e,
        Err(err) => return (0, vec![(input_dir.to_path_buf(), err.to_string())]),
    };

    fs::create_dir_all(output_dir).ok();

    let mut ok = 0;
    let mut errors: Vec<(PathBuf, String)> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let stem = path.file_stem().unwrap_or_default();
        let out_path = output_dir.join(stem).with_extension("tsp");

        match convert_file(&path, &out_path) {
            Ok(()) => ok += 1,
            Err(e) => errors.push((path, e)),
        }
    }

    (ok, errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // ── parse_discopt ────────────────────────────────────────────────────────

    #[test]
    fn test_parse_discopt_skips_first_line() {
        let input = "3\n0.0 0.0\n1.0 0.0\n0.5 1.0\n";
        let coords = parse_discopt(input).unwrap();
        assert_eq!(coords.len(), 3);
        assert_eq!(coords[0], (0.0, 0.0));
        assert_eq!(coords[1], (1.0, 0.0));
        assert_eq!(coords[2], (0.5, 1.0));
    }

    #[test]
    fn test_parse_discopt_ignores_blank_lines() {
        let input = "2\n\n1.0 2.0\n\n3.0 4.0\n";
        let coords = parse_discopt(input).unwrap();
        assert_eq!(coords.len(), 2);
    }

    #[test]
    fn test_parse_discopt_empty_after_skip_errors() {
        let input = "0\n";
        let result = parse_discopt(input);
        assert!(result.is_err(), "expected error for file with only a header line");
    }

    #[test]
    fn test_parse_discopt_bad_float_errors() {
        let input = "2\n0.0 abc\n1.0 2.0\n";
        let result = parse_discopt(input);
        assert!(result.is_err(), "expected parse error for non-numeric coordinate");
    }

    #[test]
    fn test_parse_discopt_missing_y_errors() {
        let input = "1\n1.5\n";
        let result = parse_discopt(input);
        assert!(result.is_err(), "expected error when y coordinate is absent");
    }

    // ── write_tsplib ─────────────────────────────────────────────────────────

    #[test]
    fn test_write_tsplib_header_format() {
        let coords = vec![(1.0f32, 2.0f32), (3.0, 4.0)];
        let mut buf = Vec::new();
        write_tsplib("mytest", &coords, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();

        assert!(out.contains("NAME: mytest"));
        assert!(out.contains("TYPE: TSP"));
        assert!(out.contains("DIMENSION: 2"));
        assert!(out.contains("EDGE_WEIGHT_TYPE: EUC_2D"));
        assert!(out.contains("NODE_COORD_SECTION"));
        assert!(out.contains("EOF"));
    }

    #[test]
    fn test_write_tsplib_node_ids_are_one_based() {
        let coords = vec![(0.0f32, 0.0f32), (1.0, 1.0)];
        let mut buf = Vec::new();
        write_tsplib("ids", &coords, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("\t1 "), "first node ID must be 1");
        assert!(out.contains("\t2 "), "second node ID must be 2");
        assert!(!out.contains("\t0 "), "zero-based ID must not appear");
    }

    #[test]
    fn test_write_tsplib_round_trips_through_parser() {
        use crate::tsp::tsplib;

        let coords = vec![(0.0f32, 0.0f32), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let mut buf = Vec::new();
        write_tsplib("roundtrip", &coords, &mut buf).unwrap();

        let tmp = std::env::temp_dir().join("teeline_roundtrip_test.tsp");
        fs::write(&tmp, &buf).unwrap();

        let data = tsplib::read_from_file(&tmp).expect("tsplib must parse the written file");
        assert_eq!(data.cities().len(), 4);
        assert_eq!(data.dimension(), 4);
    }

    // ── convert_file ─────────────────────────────────────────────────────────

    #[test]
    fn test_convert_file_produces_valid_tsplib() {
        use crate::tsp::tsplib;

        let tmp = std::env::temp_dir().join("teeline_cf_test");
        let input = tmp.join("input_raw");
        let output = tmp.join("output.tsp");
        fs::create_dir_all(&tmp).unwrap();
        fs::write(&input, "3\n0.0 0.0\n1.0 0.0\n0.5 1.0\n").unwrap();

        convert_file(&input, &output).expect("convert_file should succeed");

        assert!(output.exists(), "output file must be created");
        let data = tsplib::read_from_file(&output).expect("output must be valid TSPLIB");
        assert_eq!(data.cities().len(), 3);
    }

    #[test]
    fn test_convert_file_creates_missing_parent_dirs() {
        let tmp = std::env::temp_dir().join("teeline_cf_mkdir");
        let input = tmp.join("raw");
        let output = tmp.join("nested").join("deep").join("out.tsp");
        fs::create_dir_all(&tmp).unwrap();
        fs::write(&input, "2\n0.0 0.0\n1.0 1.0\n").unwrap();

        convert_file(&input, &output).expect("convert_file must create parent dirs");
        assert!(output.exists());
    }

    #[test]
    fn test_convert_file_error_on_nonexistent_input() {
        let result = convert_file(
            Path::new("/nonexistent/path/to/file"),
            Path::new("/tmp/should_not_be_created.tsp"),
        );
        assert!(result.is_err(), "must return Err for missing input file");
    }

    #[test]
    fn test_convert_file_error_on_bad_content() {
        let tmp = std::env::temp_dir().join("teeline_cf_bad");
        fs::create_dir_all(&tmp).unwrap();
        let input = tmp.join("bad");
        let output = tmp.join("bad.tsp");
        fs::write(&input, "1\n").unwrap(); // header only, no coords

        let result = convert_file(&input, &output);
        assert!(result.is_err(), "must return Err when content has no coordinates");
    }

    // ── convert_dir ──────────────────────────────────────────────────────────

    #[test]
    fn test_convert_dir_converts_all_valid_files() {
        let tmp = std::env::temp_dir().join("teeline_cd_ok");
        let input_dir = tmp.join("raw");
        let output_dir = tmp.join("out");
        fs::create_dir_all(&input_dir).unwrap();

        fs::write(input_dir.join("city_a"), "2\n0.0 0.0\n1.0 1.0\n").unwrap();
        fs::write(input_dir.join("city_b"), "2\n2.0 0.0\n3.0 1.0\n").unwrap();

        let (ok, errors) = convert_dir(&input_dir, &output_dir);
        assert_eq!(errors.len(), 0, "expected no errors: {:?}", errors);
        assert_eq!(ok, 2);
        assert!(output_dir.join("city_a.tsp").exists());
        assert!(output_dir.join("city_b.tsp").exists());
    }

    #[test]
    fn test_convert_dir_error_on_nonexistent_input_dir() {
        let (ok, errors) = convert_dir(
            Path::new("/nonexistent/input_dir"),
            Path::new("/tmp/teeline_cd_err_out"),
        );
        assert_eq!(ok, 0);
        assert!(!errors.is_empty(), "must report an error for missing directory");
    }

    #[test]
    fn test_convert_dir_reports_bad_files_in_errors() {
        let tmp = std::env::temp_dir().join("teeline_cd_mixed");
        let input_dir = tmp.join("raw");
        let output_dir = tmp.join("out");
        fs::create_dir_all(&input_dir).unwrap();

        fs::write(input_dir.join("good"), "2\n0.0 0.0\n1.0 1.0\n").unwrap();
        fs::write(input_dir.join("bad"), "1\n").unwrap(); // no coords

        let (ok, errors) = convert_dir(&input_dir, &output_dir);
        assert_eq!(ok, 1, "one file should succeed");
        assert_eq!(errors.len(), 1, "one file should fail");
    }
}
