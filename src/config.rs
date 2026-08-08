use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::tsp::{
    AcoOptions, AppOptions, CSOptions, FPAOptions, FourierOptions, GAOptions, HeuristicOptions,
    LKOptions, SAOptions, SOMOptions, Solvers,
};

/// Unifies CLI args and TOML tables as the same kind of options source.
/// Each provider takes a base and returns an overridden copy.
pub trait AppOptionsProvider {
    fn provide(&self, base: AppOptions) -> Result<AppOptions, String>;
}

/// Applies recognised sub-tables from a `toml::Table` to the base `AppOptions`.
/// Only `solver`, `sa`, `ga`, `cs`, `fpa`, `fourier`, `lk`, `som`, `aco`, and `heuristic`
/// are valid top-level keys.
/// Returns `Err` on unknown keys or mis-typed sub-tables.
pub struct TomlTableProvider<'a>(pub &'a toml::Table);

impl AppOptionsProvider for TomlTableProvider<'_> {
    fn provide(&self, mut base: AppOptions) -> Result<AppOptions, String> {
        for (key, value) in self.0.iter() {
            match key.as_str() {
                "solver" => {}
                "sa" => {
                    let t = value.as_table().ok_or("config: `sa` must be a table")?;
                    base.sa = Some(SAOptions::from_toml(t)?);
                }
                "ga" => {
                    let t = value.as_table().ok_or("config: `ga` must be a table")?;
                    base.ga = Some(GAOptions::from_toml(t)?);
                }
                "cs" => {
                    let t = value.as_table().ok_or("config: `cs` must be a table")?;
                    base.cs = Some(CSOptions::from_toml(t)?);
                }
                "fpa" => {
                    let t = value.as_table().ok_or("config: `fpa` must be a table")?;
                    base.fpa = Some(FPAOptions::from_toml(t)?);
                }
                "fourier" => {
                    let t = value
                        .as_table()
                        .ok_or("config: `fourier` must be a table")?;
                    base.fourier = Some(FourierOptions::from_toml(t)?);
                }
                "lk" => {
                    let t = value.as_table().ok_or("config: `lk` must be a table")?;
                    base.lk = Some(LKOptions::from_toml(t)?);
                }
                "som" => {
                    let t = value.as_table().ok_or("config: `som` must be a table")?;
                    base.som = Some(SOMOptions::from_toml(t)?);
                }
                "aco" => {
                    let t = value.as_table().ok_or("config: `aco` must be a table")?;
                    base.aco = Some(AcoOptions::from_toml(t)?);
                }
                "heuristic" => {
                    let t = value
                        .as_table()
                        .ok_or("config: `heuristic` must be a table")?;
                    base.heuristic = Some(HeuristicOptions::from_toml(t)?);
                }
                other => {
                    return Err(format!(
                        "config: unknown field `{other}` — valid stage fields: solver, sa, ga, cs, fpa, fourier, lk, som, aco, heuristic"
                    ));
                }
            }
        }
        Ok(base)
    }
}

/// Parse a TOML string into a list of `(Solvers, AppOptions)` pairs.
///
/// `base` is the starting `AppOptions` (usually `AppOptions::default()`).
/// Per-stage `[stage.sa]`, `[stage.ga]`, etc. are merged on top of `base`.
/// Returns `Err` on parse errors, unknown keys, or wrong sub-table for a solver.
pub fn load_pipeline_config(
    source: &str,
    base: AppOptions,
) -> Result<Vec<(Solvers, AppOptions)>, String> {
    let root: toml::Table =
        toml::from_str(source).map_err(|e| format!("config: TOML parse error: {e}"))?;

    let stage_array = root
        .get("stage")
        .ok_or("config: missing [[stage]] array — at least one stage is required")?
        .as_array()
        .ok_or("config: `stage` must be an array of tables ([[stage]])")?;

    if stage_array.is_empty() {
        return Err("config: [[stage]] list is empty — at least one stage is required".into());
    }

    let mut stages = Vec::with_capacity(stage_array.len());

    for (i, entry) in stage_array.iter().enumerate() {
        let table = entry
            .as_table()
            .ok_or_else(|| format!("config: [[stage]] entry {i} is not a table"))?;

        let solver_name = table
            .get("solver")
            .ok_or_else(|| format!("config: [[stage]] entry {i} missing required `solver` field"))?
            .as_str()
            .ok_or_else(|| format!("config: [[stage]] entry {i}: `solver` must be a string"))?;

        let solver = Solvers::from_str(solver_name)
            .map_err(|_| format!("config: [[stage]] entry {i}: unknown solver `{solver_name}`"))?;

        let stage_options = TomlTableProvider(table).provide(base.clone())?;

        // Hard error if a solver-specific sub-table is present for the wrong solver.
        for (sub, belongs) in [
            ("sa", matches!(solver, Solvers::SimulatedAnnealing)),
            ("ga", matches!(solver, Solvers::GeneticAlgorithm)),
            ("cs", matches!(solver, Solvers::CuckooSearch)),
            ("fpa", matches!(solver, Solvers::FlowerPollination)),
            ("fourier", matches!(solver, Solvers::Fourier)),
            ("lk", matches!(solver, Solvers::LinKernighan)),
            ("som", matches!(solver, Solvers::KohonenSom)),
            ("aco", matches!(solver, Solvers::AntColony)),
            (
                "heuristic",
                !matches!(
                    solver,
                    Solvers::SimulatedAnnealing
                        | Solvers::GeneticAlgorithm
                        | Solvers::CuckooSearch
                        | Solvers::FlowerPollination
                        | Solvers::Fourier
                        | Solvers::LinKernighan
                        | Solvers::KohonenSom
                        | Solvers::AntColony
                ),
            ),
        ] {
            if table.contains_key(sub) && !belongs {
                return Err(format!(
                    "config: stage {i} ({solver_name}): `[stage.{sub}]` is not valid for this solver"
                ));
            }
        }

        stages.push((solver, stage_options));
    }

    Ok(stages)
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
        (Some(_), Some(s)) if !s.is_empty() => {
            Err("--config and --steps are mutually exclusive — provide one or the other".into())
        }
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

/// A no-op `AppOptionsProvider` that leaves the base unchanged.
/// Used in tests and config-file mode where no CLI overrides apply.
pub struct IdentityProvider;

impl AppOptionsProvider for IdentityProvider {
    fn provide(&self, base: AppOptions) -> Result<AppOptions, String> {
        Ok(base)
    }
}

/// Reads a pipeline config file and returns fully-resolved `(Solvers, AppOptions)` pairs.
/// The `overrides` provider applies to the base before per-stage TOML keys are merged.
pub fn resolve_config_file<P: AppOptionsProvider>(
    path: &Path,
    overrides: &P,
) -> Result<Vec<(Solvers, AppOptions)>, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read config file '{}': {e}", path.display()))?;

    let base = overrides.provide(AppOptions::default())?;
    load_pipeline_config(&source, base)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::AppOptions;

    // --- TomlTableProvider ---

    #[test]
    fn test_toml_provider_skips_solver_key() {
        let table: toml::Table = toml::from_str("solver = \"nn\"").unwrap();
        let opts = TomlTableProvider(&table)
            .provide(AppOptions::default())
            .unwrap();
        assert!(opts.sa.is_none());
        assert!(opts.heuristic.is_none());
    }

    #[test]
    fn test_toml_provider_sa_sub_table_works() {
        let table: toml::Table = toml::from_str(
            "solver = \"sa\"\n[sa]\nepochs = 5000\ncooling_rate = 0.0005\nmax_temperature = 200.0\nmin_temperature = 0.001"
        ).unwrap();
        let opts = TomlTableProvider(&table)
            .provide(AppOptions::default())
            .unwrap();
        let sa = opts.sa.unwrap();
        assert_eq!(sa.heuristic.epochs, 5000);
        assert!((sa.cooling_rate - 0.0005).abs() < 1e-6);
    }

    #[test]
    fn test_toml_provider_fourier_sub_table_works() {
        let table: toml::Table =
            toml::from_str("solver = \"fourier\"\n[fourier]\nk_max = 32\nm = 1120").unwrap();
        let opts = TomlTableProvider(&table)
            .provide(AppOptions::default())
            .unwrap();
        let fourier = opts.fourier.unwrap();
        assert_eq!(fourier.k_max, 32);
        assert_eq!(fourier.m, 1120);
    }

    #[test]
    fn test_toml_provider_lk_sub_table_works() {
        let table: toml::Table = toml::from_str("solver = \"lk\"\n[lk]\nmax_depth = 2").unwrap();
        let opts = TomlTableProvider(&table)
            .provide(AppOptions::default())
            .unwrap();
        assert_eq!(opts.lk.unwrap().max_depth, 2);
    }

    #[test]
    fn test_toml_provider_som_sub_table_works() {
        let table: toml::Table = toml::from_str("solver = \"som\"\n[som]\nepochs = 500").unwrap();
        let opts = TomlTableProvider(&table)
            .provide(AppOptions::default())
            .unwrap();
        assert_eq!(opts.som.unwrap().epochs, 500);
    }

    #[test]
    fn test_toml_provider_heuristic_sub_table_works() {
        let table: toml::Table =
            toml::from_str("solver = \"2opt\"\n[heuristic]\nepochs = 500").unwrap();
        let opts = TomlTableProvider(&table)
            .provide(AppOptions::default())
            .unwrap();
        assert_eq!(opts.heuristic.unwrap().epochs, 500);
    }

    #[test]
    fn test_toml_provider_unknown_key_errors() {
        let table: toml::Table = toml::from_str("epoch = 9999").unwrap();
        let err = TomlTableProvider(&table)
            .provide(AppOptions::default())
            .unwrap_err();
        assert!(err.contains("unknown field"), "got: {err}");
        assert!(err.contains("epoch"), "got: {err}");
    }

    #[test]
    fn test_toml_provider_flat_epochs_errors_as_unknown_key() {
        let table: toml::Table = toml::from_str("solver = \"sa\"\nepochs = 9999").unwrap();
        let err = TomlTableProvider(&table)
            .provide(AppOptions::default())
            .unwrap_err();
        assert!(err.contains("unknown field"), "got: {err}");
    }

    // --- load_pipeline_config ---

    #[test]
    fn test_load_pipeline_config_empty_stages_errors() {
        let toml = "[global]\nepochs = 100\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(
            err.contains("[[stage]]") || err.contains("stage"),
            "got: {err}"
        );
    }

    #[test]
    fn test_load_pipeline_config_unknown_key_in_stage_errors() {
        let toml = "[[stage]]\nsolver = \"nn\"\nepoch = 999\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("unknown field"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_unknown_solver_errors() {
        let toml = "[[stage]]\nsolver = \"bogus\"\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("bogus"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_cooling_rate_zero_errors() {
        let toml = "[[stage]]\nsolver = \"sa\"\n\n[stage.sa]\ncooling_rate = 0.0\nmax_temperature = 1000.0\nmin_temperature = 0.001\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("cooling_rate"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_cooling_rate_ge_one_errors() {
        let toml = "[[stage]]\nsolver = \"sa\"\n\n[stage.sa]\ncooling_rate = 1.5\nmax_temperature = 1000.0\nmin_temperature = 0.001\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("cooling_rate"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_negative_max_temperature_errors() {
        let toml = "[[stage]]\nsolver = \"sa\"\n\n[stage.sa]\ncooling_rate = 0.001\nmax_temperature = -1.0\nmin_temperature = 0.001\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("max_temperature"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_min_ge_max_temperature_errors() {
        let toml = "[[stage]]\nsolver = \"sa\"\n\n[stage.sa]\ncooling_rate = 0.001\nmin_temperature = 500.0\nmax_temperature = 100.0\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("min_temperature"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_mutation_probability_out_of_range_errors() {
        let toml = "[[stage]]\nsolver = \"ga\"\n\n[stage.ga]\nmutation_probability = 1.5\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("mutation_probability"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_sa_sub_table_on_nn_errors() {
        let toml = "[[stage]]\nsolver = \"nn\"\n\n[stage.sa]\ncooling_rate = 0.001\nmax_temperature = 100.0\nmin_temperature = 0.001\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("sa"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_heuristic_sub_table_on_sa_errors() {
        let toml = "[[stage]]\nsolver = \"sa\"\n\n[stage.sa]\ncooling_rate = 0.001\nmax_temperature = 100.0\nmin_temperature = 0.001\n\n[stage.heuristic]\nepochs = 5000\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("heuristic"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_heuristic_sub_table_on_fourier_errors() {
        // Fourier reads only opts.fourier at solve time, never opts.heuristic —
        // [stage.heuristic] on a fourier stage is a silent no-op unless rejected here.
        let toml = "[[stage]]\nsolver = \"fourier\"\n\n[stage.fourier]\nk_max = 32\n\n[stage.heuristic]\nepochs = 500\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("heuristic"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_fourier_sub_table_on_nn_errors() {
        let toml = "[[stage]]\nsolver = \"nn\"\n\n[stage.fourier]\nk_max = 32\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("fourier"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_heuristic_sub_table_on_lk_errors() {
        // LK reads only opts.lk at solve time, never opts.heuristic —
        // [stage.heuristic] on an lk stage is a silent no-op unless rejected here.
        let toml = "[[stage]]\nsolver = \"lk\"\n\n[stage.lk]\nmax_depth = 2\n\n[stage.heuristic]\nepochs = 500\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("heuristic"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_heuristic_sub_table_on_som_errors() {
        let toml = "[[stage]]\nsolver = \"som\"\n\n[stage.som]\nepochs = 500\n\n[stage.heuristic]\nepochs = 500\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("heuristic"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_lk_sub_table_on_nn_errors() {
        let toml = "[[stage]]\nsolver = \"nn\"\n\n[stage.lk]\nmax_depth = 2\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("lk"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_som_sub_table_on_nn_errors() {
        let toml = "[[stage]]\nsolver = \"nn\"\n\n[stage.som]\nepochs = 500\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("som"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_lk_max_depth_zero_errors() {
        // Proves validate() actually fires through load_pipeline_config -> from_toml,
        // not just that the [stage.lk] table parses.
        let toml = "[[stage]]\nsolver = \"lk\"\n\n[stage.lk]\nmax_depth = 0\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("max_depth"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_som_learning_rate_out_of_range_errors() {
        let toml = "[[stage]]\nsolver = \"som\"\n\n[stage.som]\nlearning_rate = 1.5\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("learning_rate"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_irrelevant_temp_on_nn_errors() {
        // top-level cooling_rate is unknown field, not a soft warning any more
        let toml = "[[stage]]\nsolver = \"nn\"\ncooling_rate = 500.0\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("cooling_rate"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_returns_tuples() {
        let toml = "[[stage]]\nsolver = \"nn\"\n\n[[stage]]\nsolver = \"sa\"\n\n[stage.sa]\ncooling_rate = 0.001\nmin_temperature = 0.0001\nmax_temperature = 100.0\n";
        let stages = load_pipeline_config(toml, AppOptions::default()).unwrap();
        assert_eq!(stages.len(), 2);
        assert_eq!(stages[0].0, Solvers::NearestNeighbor);
        assert_eq!(stages[1].0, Solvers::SimulatedAnnealing);
        let sa_opts = stages[1].1.sa.as_ref().unwrap();
        assert!((sa_opts.cooling_rate - 0.001).abs() < 1e-6);
    }

    #[test]
    fn test_load_pipeline_config_sa_epochs_in_sub_table() {
        let toml = "[[stage]]\nsolver = \"sa\"\n\n[stage.sa]\nepochs = 5000\ncooling_rate = 0.001\nmin_temperature = 0.001\nmax_temperature = 1000.0\n";
        let stages = load_pipeline_config(toml, AppOptions::default()).unwrap();
        assert_eq!(stages[0].1.sa.as_ref().unwrap().heuristic.epochs, 5000);
    }

    #[test]
    fn test_load_pipeline_config_heuristic_epochs_for_generic_solver() {
        let toml = "[[stage]]\nsolver = \"2opt\"\n\n[stage.heuristic]\nepochs = 200\n";
        let stages = load_pipeline_config(toml, AppOptions::default()).unwrap();
        assert_eq!(stages[0].1.heuristic.as_ref().unwrap().epochs, 200);
    }

    #[test]
    fn test_load_pipeline_config_ga_mutation_in_sub_table() {
        let toml =
            "[[stage]]\nsolver = \"ga\"\n\n[stage.ga]\nmutation_probability = 0.05\nepochs = 100\n";
        let stages = load_pipeline_config(toml, AppOptions::default()).unwrap();
        let ga = stages[0].1.ga.as_ref().unwrap();
        assert!((ga.mutation_probability - 0.05).abs() < 1e-6);
        assert_eq!(ga.heuristic.epochs, 100);
    }

    #[test]
    fn test_load_pipeline_config_sa_uses_from_toml_validation() {
        // validate() is called inside SAOptions::from_toml()
        let toml = "[[stage]]\nsolver = \"sa\"\n\n[stage.sa]\ncooling_rate = 0.0005\nmax_temperature = 200.0\nmin_temperature = 0.001\n";
        let stages = load_pipeline_config(toml, AppOptions::default()).unwrap();
        let sa = stages[0].1.sa.as_ref().unwrap();
        assert!((sa.max_temperature - 200.0).abs() < 0.01);
    }

    #[test]
    fn test_toml_provider_aco_sub_table_works() {
        let table: toml::Table =
            toml::from_str("solver = \"aco\"\n[aco]\nnum_ants = 10\nbeta = 3.0").unwrap();
        let opts = TomlTableProvider(&table)
            .provide(AppOptions::default())
            .unwrap();
        let aco = opts.aco.unwrap();
        assert_eq!(aco.num_ants, 10);
        assert!((aco.beta - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_load_pipeline_config_aco_sub_table_on_nn_errors() {
        let toml = "[[stage]]\nsolver = \"nn\"\n\n[stage.aco]\nnum_ants = 10\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("aco"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_heuristic_sub_table_on_aco_errors() {
        // ACO reads only opts.aco at solve time, never opts.heuristic —
        // [stage.heuristic] on an aco stage is a silent no-op unless rejected here.
        let toml = "[[stage]]\nsolver = \"aco\"\n\n[stage.aco]\nnum_ants = 10\n\n[stage.heuristic]\nepochs = 500\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("heuristic"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_aco_evaporation_rate_zero_errors() {
        // Proves validate() actually fires through load_pipeline_config -> from_toml.
        let toml = "[[stage]]\nsolver = \"aco\"\n\n[stage.aco]\nevaporation_rate = 0.0\n";
        let err = load_pipeline_config(toml, AppOptions::default()).unwrap_err();
        assert!(err.contains("evaporation_rate"), "got: {err}");
    }

    #[test]
    fn test_load_pipeline_config_aco_num_ants_in_sub_table() {
        let toml = "[[stage]]\nsolver = \"aco\"\n\n[stage.aco]\nnum_ants = 15\nalpha = 2.0\n";
        let stages = load_pipeline_config(toml, AppOptions::default()).unwrap();
        assert_eq!(stages[0].0, Solvers::AntColony);
        let aco = stages[0].1.aco.as_ref().unwrap();
        assert_eq!(aco.num_ants, 15);
        assert!((aco.alpha - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_load_pipeline_config_greedy_edge_stage_parses() {
        let toml = "[[stage]]\nsolver = \"greedy_edge\"\n";
        let stages = load_pipeline_config(toml, AppOptions::default()).unwrap();
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0].0, Solvers::GreedyEdge);
    }

    #[test]
    fn test_load_pipeline_config_greedy_edge_as_pipeline_seed() {
        // greedy_edge is a plain-HeuristicOptions solver like nn/two_opt/christofides —
        // it uses the generic `[stage.heuristic]` sub-table (accepted but unused at
        // solve time), and its short alias `gec` must resolve too.
        let toml = "[[stage]]\nsolver = \"gec\"\n\n[[stage]]\nsolver = \"two_opt\"\n";
        let stages = load_pipeline_config(toml, AppOptions::default()).unwrap();
        assert_eq!(stages.len(), 2);
        assert_eq!(stages[0].0, Solvers::GreedyEdge);
        assert_eq!(stages[1].0, Solvers::TwoOpt);
    }
}
