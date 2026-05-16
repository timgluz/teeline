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

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
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
    fn test_write_tsplib_round_trips_through_parser() {
        use crate::tsp::tsplib;

        let coords = vec![(0.0f32, 0.0f32), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let mut buf = Vec::new();
        write_tsplib("roundtrip", &coords, &mut buf).unwrap();

        let tmp = std::env::temp_dir().join("teeline_roundtrip_test.tsp");
        std::fs::write(&tmp, &buf).unwrap();

        let data = tsplib::read_from_file(&tmp).expect("tsplib must parse the written file");
        assert_eq!(data.cities().len(), 4);
        assert_eq!(data.dimension(), 4);
    }
}
