//! Private, offline diagnostics against retained individual simulation samples.
use super::VerifiedBattleInput;
use combat_core::Simulator;
use combat_types::{CombatOutcome, CombatRequest, SimulationResult};
use serde::Serialize;
use serde_json::Value;
use std::fmt::Write as _;

const THRESHOLD: f64 = 0.05;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonStatus {
    Unremarkable,
    Suspicious,
    StatisticallyUncertain,
    NotAssessable,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbabilityInterval {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct NumericSpread {
    pub mean: f64,
    pub median: f64,
    pub minimum: u64,
    pub maximum: u64,
    pub standard_deviation: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricComparison {
    pub name: String,
    pub observation: Value,
    pub run_count: usize,
    pub occurrence_count: Option<usize>,
    pub probability: Option<f64>,
    pub spread: Option<NumericSpread>,
    pub interval: Option<ProbabilityInterval>,
    pub threshold: f64,
    pub status: ComparisonStatus,
    pub explanation: String,
}

#[derive(Debug, Serialize)]
pub struct BattleComparison {
    pub stages: Vec<ComparisonStage>,
    pub run_count: usize,
    pub metrics: Vec<MetricComparison>,
    pub method: String,
    pub software_version: String,
    pub diagnostics: super::ComparisonDiagnostics,
}

#[derive(Debug, Serialize)]
pub struct ComparisonStage {
    pub run_count: usize,
    pub metrics: Vec<MetricComparison>,
}

/// Run the same shared diagnostic workflow with the real offline simulator.
pub fn compare_battle(input: &VerifiedBattleInput) -> Result<BattleComparison, &'static str> {
    let simulator = Simulator::new();
    compare_with(input, |request| {
        simulator.simulate_multiple(request).results
    })
}

/// Inject whole batches at the simulation boundary. Batches must contain exactly
/// the requested number of individual results; no samples are discarded.
pub fn compare_with(
    input: &VerifiedBattleInput,
    mut simulate: impl FnMut(&CombatRequest) -> Vec<SimulationResult>,
) -> Result<BattleComparison, &'static str> {
    let sampling = bounded_samples(
        |count| {
            let mut request = input.request.clone();
            request.simulations = count as u32;
            Ok(simulate(&request))
        },
        |samples| assess(input, samples),
    )?;
    Ok(BattleComparison {
        stages: sampling.stages,
        run_count: sampling.run_count, metrics: sampling.metrics,
        diagnostics: super::comparison_diagnostics::diagnostics(input),
        method: "Inclusive empirical two-sided tail: min(1, 2 * min(P(X <= observed), P(X >= observed))). Numeric intervals double and cap the 95% Wilson interval of the smaller inclusive tail; categorical intervals use occurrence counts directly. Individual 5% labels are diagnostics, not a multiple-testing-adjusted proof or a global correctness pass. Samples are retained at totals 50, 200, 1000; suspicious labels do not request reruns.".to_owned(),
        software_version: env!("CARGO_PKG_VERSION").to_owned(),
    })
}

pub(super) struct SamplingResult {
    pub run_count: usize,
    pub stages: Vec<ComparisonStage>,
    pub metrics: Vec<MetricComparison>,
}

pub(super) fn bounded_samples<T>(
    mut simulate: impl FnMut(usize) -> Result<Vec<T>, &'static str>,
    assess: impl Fn(&[T]) -> Vec<MetricComparison>,
) -> Result<SamplingResult, &'static str> {
    let mut samples = Vec::new();
    let mut stages = Vec::new();
    let mut metrics = assess(&samples);
    for total in [50, 200, 1000] {
        if total != 50
            && !metrics
                .iter()
                .any(|m| m.status == ComparisonStatus::StatisticallyUncertain)
        {
            break;
        }
        let count = total - samples.len();
        let batch = simulate(count)?;
        if batch.len() != count {
            return Err("simulation batch did not return the requested sample count");
        }
        samples.extend(batch);
        metrics = assess(&samples);
        stages.push(ComparisonStage {
            run_count: samples.len(),
            metrics: metrics.clone(),
        });
    }
    Ok(SamplingResult {
        run_count: samples.len(),
        stages,
        metrics,
    })
}

fn assess(input: &VerifiedBattleInput, samples: &[SimulationResult]) -> Vec<MetricComparison> {
    let observed = input.observed.as_ref().unwrap_or(&Value::Null);
    let winner = observed
        .get("winner")
        .and_then(Value::as_str)
        .and_then(|s| match s {
            "attacker" => Some(CombatOutcome::AttackersWin),
            "defender" => Some(CombatOutcome::DefendersWin),
            "draw" => Some(CombatOutcome::Draw),
            _ => None,
        });
    let mut outcome = omitted("outcome", "no recognized observed outcome", samples.len());
    if let Some(winner) = winner {
        outcome.observation = observed["winner"].clone();
        if !samples.is_empty() {
            let count = samples.iter().filter(|s| s.outcome == winner).count();
            outcome.occurrence_count = Some(count);
            assess_probability(&mut outcome, count, 1.0);
        }
    }
    let mut metrics = vec![
        outcome,
        numeric(
            "rounds",
            observed.get("combat_rounds").and_then(Value::as_u64),
            samples.iter().map(|s| u64::from(s.rounds)).collect(),
        ),
    ];
    metrics.extend(super::comparison_metrics::losses(input, samples));
    metrics.extend(super::comparison_metrics::debris(input, samples));
    for (name, field, reason) in [
        (
            "loot",
            "loot_metal",
            "loot inputs and observation semantics are not verified",
        ),
        (
            "repaired_defenses",
            "repaired_defenses",
            "post-combat defence rebuild is not modeled",
        ),
        (
            "wreck_field",
            "wreck_field",
            "post-combat wreck fields are not modeled",
        ),
        (
            "moon_chance",
            "moon_chance",
            "moon chance comparison is outside the supported metric set",
        ),
    ] {
        let mut metric = omitted(name, reason, samples.len());
        // Retain only numeric evidence, never arbitrary report text or fields.
        metric.observation = super::comparison_metrics::numeric_evidence(&observed[field]);
        metrics.push(metric);
    }
    metrics
}

pub(super) fn omitted(name: &str, reason: &str, run_count: usize) -> MetricComparison {
    MetricComparison {
        name: name.to_owned(),
        observation: Value::Null,
        run_count,
        occurrence_count: None,
        probability: None,
        spread: None,
        interval: None,
        threshold: THRESHOLD,
        status: ComparisonStatus::NotAssessable,
        explanation: reason.to_owned(),
    }
}

pub(super) fn numeric(
    name: &str,
    observation: Option<u64>,
    mut values: Vec<u64>,
) -> MetricComparison {
    let mut result = omitted(name, "matching observed evidence is missing", values.len());
    let Some(observation) = observation else {
        return result;
    };
    result.observation = observation.into();
    if values.is_empty() {
        return result;
    }
    let below = values.iter().filter(|&&v| v <= observation).count();
    let above = values.iter().filter(|&&v| v >= observation).count();
    let mean = values.iter().map(|&v| v as f64).sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|&v| (v as f64 - mean).powi(2))
        .sum::<f64>()
        / values.len() as f64;
    values.sort_unstable();
    let middle = values.len() / 2;
    result.spread = Some(NumericSpread {
        mean,
        median: f64::midpoint(values[(values.len() - 1) / 2] as f64, values[middle] as f64),
        minimum: values[0],
        maximum: values[values.len() - 1],
        standard_deviation: variance.sqrt(),
    });
    assess_probability(&mut result, below.min(above), 2.0);
    result
}

pub(super) fn assess_probability(result: &mut MetricComparison, count: usize, multiplier: f64) {
    let n = result.run_count as f64;
    let p = count as f64 / n;
    let z: f64 = 1.959_963_984_540_054;
    let denominator = 1.0 + z * z / n;
    let center = (p + z * z / (2.0 * n)) / denominator;
    let radius = z * (p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt() / denominator;
    let interval = ProbabilityInterval {
        lower: (multiplier * (center - radius)).clamp(0.0, 1.0),
        upper: (multiplier * (center + radius)).clamp(0.0, 1.0),
    };
    result.probability = Some((multiplier * p).min(1.0));
    result.status = if interval.lower >= THRESHOLD {
        ComparisonStatus::Unremarkable
    } else if interval.upper < THRESHOLD {
        ComparisonStatus::Suspicious
    } else {
        ComparisonStatus::StatisticallyUncertain
    };
    "Rarity under the supplied inputs and current combat model; not proof of correctness or a reason to tune inputs.".clone_into(&mut result.explanation);
    result.interval = Some(interval);
}

impl BattleComparison {
    /// Human-readable counterpart of the structured result used by the CLI.
    #[must_use]
    pub fn render_text(&self) -> String {
        let mut output = format!(
            "Private battle comparison: {} simulations\n{}\nSoftware version: {}\n",
            self.run_count, self.method, self.software_version
        );
        for stage in self.stages.iter().filter(|s| s.run_count < self.run_count) {
            for metric in stage
                .metrics
                .iter()
                .filter(|m| m.status == ComparisonStatus::Suspicious)
            {
                let _ = writeln!(
                    output,
                    "Earlier suspicious assessment at {} simulations: {} (retained in stage history)",
                    stage.run_count, metric.name
                );
            }
        }
        for metric in &self.metrics {
            let status = match metric.status {
                ComparisonStatus::Unremarkable => "unremarkable",
                ComparisonStatus::Suspicious => "suspicious",
                ComparisonStatus::StatisticallyUncertain => "statistically_uncertain",
                ComparisonStatus::NotAssessable => "not_assessable",
            };
            let _ = writeln!(
                output,
                "\n{}: {status}\n  observed: {}; runs: {}; rarity threshold: {:.0}%",
                metric.name,
                metric.observation,
                metric.run_count,
                metric.threshold * 100.0
            );
            if let (Some(p), Some(interval)) = (metric.probability, &metric.interval) {
                let _ = writeln!(
                    output,
                    "  probability: {:.4}; 95% Wilson interval: [{:.4}, {:.4}]",
                    p, interval.lower, interval.upper
                );
            }
            if let Some(count) = metric.occurrence_count {
                let _ = writeln!(
                    output,
                    "  observed outcome occurred {count}/{} times",
                    metric.run_count
                );
            }
            if let Some(spread) = &metric.spread {
                let _ = writeln!(
                    output,
                    "  mean: {:.2}; median: {:.2}; range: {}..{}; standard deviation: {:.2}",
                    spread.mean,
                    spread.median,
                    spread.minimum,
                    spread.maximum,
                    spread.standard_deviation
                );
            }
            let _ = writeln!(output, "  {}", metric.explanation);
        }
        let _ = writeln!(
            output,
            "\nStarting statistics, evidence provenance and model context:\n{}",
            serde_json::to_string_pretty(&self.diagnostics).unwrap_or_default()
        );
        output
    }
}
