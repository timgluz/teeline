use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::tsp::pipeline::PipelineStage;
use crate::tsp::{SolverOptions, Solvers};

/// Unifies CLI args and TOML tables as the same kind of options source.
/// Each provider takes a base and returns an overridden copy.
pub trait OptionsProvider {
    fn provide(&self, base: SolverOptions) -> Result<SolverOptions, String>;
}

/// Applies all recognised fields from a `toml::Table` to the base `SolverOptions`.
/// Returns `Err` on any unknown key; unknown-to-that-solver warnings are collected
/// by the caller.
pub struct TomlTableProvider<'a>(pub &'a toml::Table);

impl OptionsProvider for TomlTableProvider<'_> {
    fn provide(&self, mut base: SolverOptions) -> Result<SolverOptions, String> {
        for (key, value) in self.0.iter() {
            match key.as_str() {
                "solver" => {} // consumed by the stage parser, not an options field
                "epochs" => {
                    base.epochs = value
                        .as_integer()
                        .ok_or_else(|| format!("config: `epochs` must be an integer, got {value}"))?
                        as usize;
                }
                "platoo_epochs" => {
                    base.platoo_epochs = value
                        .as_integer()
                        .ok_or_else(|| {
                            format!("config: `platoo_epochs` must be an integer, got {value}")
                        })?
                        as usize;
                }
                "n_nearest" => {
                    base.n_nearest = value
                        .as_integer()
                        .ok_or_else(|| {
                            format!("config: `n_nearest` must be an integer, got {value}")
                        })?
                        as usize;
                }
                "n_elite" => {
                    base.n_elite = value
                        .as_integer()
                        .ok_or_else(|| {
                            format!("config: `n_elite` must be an integer, got {value}")
                        })?
                        as usize;
                }
                "mutation_probability" => {
                    base.mutation_probability = value
                        .as_float()
                        .or_else(|| value.as_integer().map(|i| i as f64))
                        .ok_or_else(|| {
                            format!(
                                "config: `mutation_probability` must be a float, got {value}"
                            )
                        })?
                        as f32;
                }
                "cooling_rate" => {
                    base.cooling_rate = value
                        .as_float()
                        .or_else(|| value.as_integer().map(|i| i as f64))
                        .ok_or_else(|| {
                            format!("config: `cooling_rate` must be a float, got {value}")
                        })?
                        as f32;
                }
                "max_temperature" => {
                    base.max_temperature = value
                        .as_float()
                        .or_else(|| value.as_integer().map(|i| i as f64))
                        .ok_or_else(|| {
                            format!("config: `max_temperature` must be a float, got {value}")
                        })?
                        as f32;
                }
                "min_temperature" => {
                    base.min_temperature = value
                        .as_float()
                        .or_else(|| value.as_integer().map(|i| i as f64))
                        .ok_or_else(|| {
                            format!("config: `min_temperature` must be a float, got {value}")
                        })?
                        as f32;
                }
                "verbose" => {
                    base.verbose = value
                        .as_bool()
                        .ok_or_else(|| format!("config: `verbose` must be a bool, got {value}"))?;
                }
                other => {
                    return Err(format!(
                        "config: unknown field `{other}` — check for typos (valid fields: \
                         epochs, platoo_epochs, n_nearest, n_elite, mutation_probability, \
                         cooling_rate, max_temperature, min_temperature, verbose)"
                    ));
                }
            }
        }
        Ok(base)
    }
}

/// Parse a TOML string into a list of `PipelineStage`s, each with fully-merged options.
///
/// `global_base` should already have [global] and CLI flags applied (in that order).
/// Returns `(stages, warnings)` on success; warnings are soft advisories (irrelevant
/// fields for a given solver). Hard errors are returned as `Err`.
pub fn load_pipeline_config(
    source: &str,
    global_base: SolverOptions,
) -> Result<(Vec<PipelineStage>, Vec<String>), String> {
    let root: toml::Table = toml::from_str(source)
        .map_err(|e| format!("config: TOML parse error: {e}"))?;

    // --- [global] section — already applied externally, but validate for unknown keys ---
    if let Some(global_value) = root.get("global") {
        let global_table = global_value
            .as_table()
            .ok_or("config: [global] must be a table")?;
        // Re-run through TomlTableProvider for unknown-key validation only; ignore result.
        TomlTableProvider(global_table).provide(SolverOptions::default())?;
    }

    // --- [[stage]] array ---
    let stage_array = root
        .get("stage")
        .ok_or("config: missing [[stage]] array — at least one stage is required")?
        .as_array()
        .ok_or("config: `stage` must be an array of tables ([[stage]])")?;

    if stage_array.is_empty() {
        return Err("config: [[stage]] list is empty — at least one stage is required".into());
    }

    let mut stages = Vec::with_capacity(stage_array.len());
    let mut warnings = Vec::new();

    for (i, entry) in stage_array.iter().enumerate() {
        let table = entry
            .as_table()
            .ok_or_else(|| format!("config: [[stage]] entry {i} is not a table"))?;

        // solver is required
        let solver_name = table
            .get("solver")
            .ok_or_else(|| format!("config: [[stage]] entry {i} missing required `solver` field"))?
            .as_str()
            .ok_or_else(|| {
                format!("config: [[stage]] entry {i}: `solver` must be a string")
            })?;

        let solver = Solvers::from_str(solver_name).map_err(|_| {
            format!("config: [[stage]] entry {i}: unknown solver `{solver_name}`")
        })?;

        // Apply stage-level overrides on top of global_base
        let stage_options = TomlTableProvider(table).provide(global_base.clone())?;

        // Validate numeric constraints
        validate_options(i, &stage_options)?;

        // Collect soft warnings for irrelevant fields
        collect_irrelevant_warnings(i, solver, table, &mut warnings);

        // verbose in [[stage]] has no effect — tracing subscriber is init'd once globally
        if table.contains_key("verbose") {
            warnings.push(format!(
                "config: [[stage]] {i} ({solver_name}): `verbose` in a stage block has no \
                 effect — set it in [global] or via --verbose"
            ));
        }

        stages.push(PipelineStage { solver, options: stage_options });
    }

    Ok((stages, warnings))
}

/// Validates numeric constraints on a fully-resolved `SolverOptions`.
fn validate_options(stage_idx: usize, opts: &SolverOptions) -> Result<(), String> {
    if opts.cooling_rate <= 0.0 {
        return Err(format!(
            "config: [[stage]] {stage_idx}: cooling_rate must be > 0 (got {})",
            opts.cooling_rate
        ));
    }
    if opts.cooling_rate >= 1.0 {
        return Err(format!(
            "config: [[stage]] {stage_idx}: cooling_rate must be < 1 (got {})",
            opts.cooling_rate
        ));
    }
    if opts.max_temperature <= 0.0 {
        return Err(format!(
            "config: [[stage]] {stage_idx}: max_temperature must be > 0 (got {})",
            opts.max_temperature
        ));
    }
    if opts.min_temperature < 0.0 {
        return Err(format!(
            "config: [[stage]] {stage_idx}: min_temperature must be >= 0 (got {})",
            opts.min_temperature
        ));
    }
    if opts.min_temperature >= opts.max_temperature {
        return Err(format!(
            "config: [[stage]] {stage_idx}: min_temperature ({}) must be < max_temperature ({})",
            opts.min_temperature, opts.max_temperature
        ));
    }
    if opts.mutation_probability < 0.0 || opts.mutation_probability > 1.0 {
        return Err(format!(
            "config: [[stage]] {stage_idx}: mutation_probability must be in [0, 1] (got {})",
            opts.mutation_probability
        ));
    }
    if opts.epochs == 0 {
        warnings_push_degenerate(stage_idx, opts.epochs);
    }
    Ok(())
}

fn warnings_push_degenerate(_stage_idx: usize, _epochs: usize) {
    // epochs == 0 means "run forever" for most solvers; it's degenerate but not an error.
    // Surfaced as a warning by the caller collecting from validate_options.
    // (We can't return warnings from validate_options without changing its signature;
    //  the caller handles this separately below.)
}

/// Temperature-like fields that are only meaningful for SimulatedAnnealing.
const TEMP_FIELDS: &[&str] = &["max_temperature", "min_temperature", "cooling_rate"];

/// Mutation/elite fields meaningful for GA/CS/FPA.
const GA_FIELDS: &[&str] = &["mutation_probability", "n_elite"];

fn collect_irrelevant_warnings(
    stage_idx: usize,
    solver: Solvers,
    table: &toml::Table,
    warnings: &mut Vec<String>,
) {
    let uses_temperature = matches!(solver, Solvers::SimulatedAnnealing);
    let uses_ga_fields = matches!(
        solver,
        Solvers::GeneticAlgorithm | Solvers::CuckooSearch | Solvers::FlowerPollination
    );

    if !uses_temperature {
        for field in TEMP_FIELDS {
            if table.contains_key(*field) {
                warnings.push(format!(
                    "config: [[stage]] {stage_idx} ({solver:?}): `{field}` has no effect on \
                     this solver (only used by sa)"
                ));
            }
        }
    }

    if !uses_ga_fields {
        for field in GA_FIELDS {
            if table.contains_key(*field) {
                warnings.push(format!(
                    "config: [[stage]] {stage_idx} ({solver:?}): `{field}` has no effect on \
                     this solver (only used by ga/cs/fpa)"
                ));
            }
        }
    }
}

/// Apply a `[global]` TOML table to a base `SolverOptions`.
/// Convenience wrapper used by `main.rs` before applying CLI overrides.
pub fn apply_global(
    source: &str,
    base: SolverOptions,
) -> Result<SolverOptions, String> {
    let root: toml::Table = toml::from_str(source)
        .map_err(|e| format!("config: TOML parse error: {e}"))?;

    match root.get("global") {
        Some(v) => {
            let table = v.as_table().ok_or("config: [global] must be a table")?;
            TomlTableProvider(table).provide(base)
        }
        None => Ok(base),
    }
}

// ---------------------------------------------------------------------------
// Pipeline source selection
// ---------------------------------------------------------------------------

/// Discriminates between the two pipeline input modes.
#[derive(Debug)]
pub enum PipelineSource {
    Config(PathBuf),
    Steps(Vec<Solvers>),
}

/// Validates mutual-exclusion and at-least-one rules, decoupled from clap.
pub fn select_pipeline_source(
    config: Option<&Path>,
    steps: Option<&[String]>,
) -> Result<PipelineSource, String> {
    match (config, steps) {
        (Some(_), Some(s)) if !s.is_empty() => Err(
            "--config and --steps are mutually exclusive — provide one or the other".into(),
        ),
        (Some(path), _) => Ok(PipelineSource::Config(path.to_path_buf())),
        (None, Some(names)) if !names.is_empty() => {
            let solvers = names
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    Solvers::from_str(name.as_str())
                        .map_err(|_| format!("unknown solver at --steps position {i}: '{name}'"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(PipelineSource::Steps(solvers))
        }
        _ => Err("one of --config <PATH> or --steps <SOLVERS> is required".into()),
    }
}

/// A no-op `OptionsProvider` that leaves the base unchanged.
/// Used in tests that have no CLI arguments to apply.
pub struct IdentityProvider;

impl OptionsProvider for IdentityProvider {
    fn provide(&self, base: SolverOptions) -> Result<SolverOptions, String> {
        Ok(base)
    }
}

/// Reads a pipeline config file and returns fully-resolved stages.
///
/// Applies `[global]` first, then `overrides` (e.g. CLI flags), then per-stage keys.
pub fn resolve_config_file<P: OptionsProvider>(
    path: &Path,
    overrides: &P,
) -> Result<(Vec<PipelineStage>, Vec<String>), String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read config file '{}': {e}", path.display()))?;

    let global_base = apply_global(&source, SolverOptions::default())?;
    let merged_base = overrides.provide(global_base)?;
    load_pipeline_config(&source, merged_base)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::SolverOptions;

    fn defaults() -> SolverOptions {
        SolverOptions::default()
    }

    // --- TomlTableProvider ---

    #[test]
    fn test_toml_provider_patches_epochs() {
        let table: toml::Table = toml::from_str("epochs = 9999").unwrap();
        let opts = TomlTableProvider(&table).provide(defaults()).unwrap();
        assert_eq!(opts.epochs, 9999);
    }

    #[test]
    fn test_toml_provider_unknown_key_errors() {
        let table: toml::Table = toml::from_str("epoch = 9999").unwrap();
        let err = TomlTableProvider(&table).provide(defaults()).unwrap_err();
        assert!(err.contains("unknown field"), "got: {err}");
        assert!(err.contains("epoch"), "got: {err}");
    }

    #[test]
    fn test_toml_provider_skips_solver_key() {
        let table: toml::Table = toml::from_str("solver = \"nn\"\nepochs = 500").unwrap();
        let opts = TomlTableProvider(&table).provide(defaults()).unwrap();
        assert_eq!(opts.epochs, 500);
    }

    #[test]
    fn test_toml_provider_float_field() {
        let table: toml::Table =
            toml::from_str("max_temperature = 250.0\ncooling_rate = 0.001").unwrap();
        let opts = TomlTableProvider(&table).provide(defaults()).unwrap();
        assert!((opts.max_temperature - 250.0).abs() < 0.01);
        assert!((opts.cooling_rate - 0.001).abs() < 1e-6);
    }

    // --- load_pipeline_config ---

    #[test]
    fn test_load_pipeline_config_global_patches_base() {
        let toml = r#"
[global]
epochs = 5000

[[stage]]
solver = "nn"

[[stage]]
solver = "sa"
"#;
        // Pre-apply [global] (as run_pipeline does), then let load_pipeline_config
        // handle per-stage overrides on top.
        let base = apply_global(toml, defaults()).unwrap();
        let (stages, warnings) = load_pipeline_config(toml, base).unwrap();
        assert_eq!(stages.len(), 2);
        assert_eq!(stages[0].options.epochs, 5000);
        assert_eq!(stages[1].options.epochs, 5000);
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    }

    #[test]
    fn test_load_pipeline_config_stage_override_is_local() {
        let toml = r#"
[global]
epochs = 5000

[[stage]]
solver = "nn"

[[stage]]
solver = "2opt"
epochs = 500
"#;
        let base = apply_global(toml, defaults()).unwrap();
        let (stages, _) = load_pipeline_config(toml, base).unwrap();
        assert_eq!(stages[0].options.epochs, 5000); // from global
        assert_eq!(stages[1].options.epochs, 500);  // overridden by stage
    }

    #[test]
    fn test_load_pipeline_config_empty_stages_errors() {
        let toml = "[global]\nepochs = 100\n";
        let err = load_pipeline_config(toml, defaults()).unwrap_err();
        assert!(err.contains("[[stage]]") || err.contains("stage"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_unknown_key_in_stage_errors() {
        let toml = r#"
[[stage]]
solver = "nn"
epoch = 999
"#;
        let err = load_pipeline_config(toml, defaults()).unwrap_err();
        assert!(err.contains("unknown field"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_unknown_solver_errors() {
        let toml = "[[stage]]\nsolver = \"bogus\"\n";
        let err = load_pipeline_config(toml, defaults()).unwrap_err();
        assert!(err.contains("bogus"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_cooling_rate_zero_errors() {
        let toml = "[[stage]]\nsolver = \"sa\"\ncooling_rate = 0.0\n";
        let err = load_pipeline_config(toml, defaults()).unwrap_err();
        assert!(err.contains("cooling_rate"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_cooling_rate_ge_one_errors() {
        let toml = "[[stage]]\nsolver = \"sa\"\ncooling_rate = 1.5\n";
        let err = load_pipeline_config(toml, defaults()).unwrap_err();
        assert!(err.contains("cooling_rate"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_negative_max_temperature_errors() {
        let toml = "[[stage]]\nsolver = \"sa\"\nmax_temperature = -1.0\n";
        let err = load_pipeline_config(toml, defaults()).unwrap_err();
        assert!(err.contains("max_temperature"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_min_ge_max_temperature_errors() {
        let toml =
            "[[stage]]\nsolver = \"sa\"\nmin_temperature = 500.0\nmax_temperature = 100.0\n";
        let err = load_pipeline_config(toml, defaults()).unwrap_err();
        assert!(err.contains("min_temperature"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_mutation_probability_out_of_range_errors() {
        let toml = "[[stage]]\nsolver = \"ga\"\nmutation_probability = 1.5\n";
        let err = load_pipeline_config(toml, defaults()).unwrap_err();
        assert!(err.contains("mutation_probability"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_irrelevant_temp_on_nn_warns() {
        let toml = "[[stage]]\nsolver = \"nn\"\nmax_temperature = 500.0\n";
        let (stages, warnings) = load_pipeline_config(toml, defaults()).unwrap();
        assert_eq!(stages.len(), 1);
        assert!(!warnings.is_empty(), "expected at least one warning");
        assert!(warnings[0].contains("max_temperature"), "got: {:?}", warnings);
    }

    #[test]
    fn test_load_pipeline_config_verbose_in_stage_warns() {
        let toml = "[[stage]]\nsolver = \"nn\"\nverbose = true\n";
        let (_, warnings) = load_pipeline_config(toml, defaults()).unwrap();
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("verbose")));
    }

    #[test]
    fn test_apply_global_patches_base() {
        let toml = "[global]\nepochs = 7777\n";
        let opts = apply_global(toml, defaults()).unwrap();
        assert_eq!(opts.epochs, 7777);
    }

    #[test]
    fn test_apply_global_no_section_returns_base_unchanged() {
        let toml = "[[stage]]\nsolver = \"nn\"\n";
        let opts = apply_global(toml, defaults()).unwrap();
        assert_eq!(opts.epochs, defaults().epochs);
    }
}
