use std::{collections::HashSet, future::Future, pin::Pin, time::Duration};

use lirays_scada_proto::namespace::v1::{Value, VarIdValue, value::Typed};
use rand::RngExt;
use serde::Deserialize;
use tokio::time::{Instant, Interval, MissedTickBehavior};

use super::{IngestionBatch, Ingestor, IngestorPlugin};

pub struct SimulatorPlugin;

/// Common timing configuration for simulator implementations, in milliseconds.
#[derive(Deserialize)]
#[serde(default)]
pub struct SimulatorConfig {
    pub interval_ms: u64,
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self { interval_ms: 1000 }
    }
}

impl SimulatorConfig {
    pub fn create_interval(&self) -> Result<Interval, String> {
        let period = Duration::from_millis(self.interval_ms);
        let start = Instant::now();
        if period.is_zero() || start.checked_add(period).is_none() {
            return Err(
                "interval_ms must be a positive, supported duration in milliseconds".into(),
            );
        }
        let mut interval = tokio::time::interval_at(start, period);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        Ok(interval)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SimulatorInstanceConfig {
    #[serde(flatten)]
    simulator: SimulatorConfig,
    variables: Vec<VariableConfig>,
}

#[derive(Clone, Copy, Default, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Waveform {
    #[default]
    Random,
    Sine,
    Square,
    Ramp,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VariableConfig {
    path: String,
    #[serde(default)]
    waveform: Waveform,
    period_ms: Option<u64>,
    #[serde(default)]
    phase_degrees: f64,
    min: f64,
    max: f64,
    decimal_places: Option<u32>,
}

struct DecimalRange {
    scale: f64,
    min: i64,
    max: i64,
}

struct SimulatedVariable {
    config: VariableConfig,
    decimals: Option<DecimalRange>,
}

struct Simulator {
    variables: Vec<SimulatedVariable>,
    interval: Interval,
    started_at: Option<Instant>,
}

impl IngestorPlugin for SimulatorPlugin {
    fn kind(&self) -> &'static str {
        "simulator"
    }

    fn create(
        &self,
        _instance_id: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn Ingestor>, String> {
        let config: SimulatorInstanceConfig =
            serde_json::from_value(config.clone()).map_err(|e| e.to_string())?;
        if config.variables.is_empty() {
            return Err("variables must contain at least one variable".into());
        }
        let mut paths = HashSet::new();
        let mut variables = Vec::with_capacity(config.variables.len());
        for variable in config.variables {
            if !variable.path.starts_with('/')
                || variable.path.contains('\0')
                || variable.path != crate::rtdata::utils::normalize_path(&variable.path)
            {
                return Err(format!("Invalid namespace path: {:?}", variable.path));
            }
            if !paths.insert(variable.path.clone()) {
                return Err(format!("Duplicate variable path: {}", variable.path));
            }
            if !variable.min.is_finite()
                || !variable.max.is_finite()
                || variable.min > variable.max
                || !(variable.max - variable.min).is_finite()
            {
                return Err(
                    "min and max must be finite, min <= max, and their difference finite".into(),
                );
            }
            if !variable.phase_degrees.is_finite() {
                return Err(format!("phase_degrees must be finite: {}", variable.path));
            }
            if variable.waveform == Waveform::Random {
                if variable.period_ms.is_some() || variable.phase_degrees != 0.0 {
                    return Err(format!(
                        "period_ms and phase_degrees apply only to periodic waveforms: {}",
                        variable.path
                    ));
                }
            } else if !variable.period_ms.is_some_and(|period| period > 0) {
                return Err(format!(
                    "A positive period_ms is required: {}",
                    variable.path
                ));
            }
            let decimals = if let Some(places) = variable.decimal_places {
                if places > 15 {
                    return Err("decimal_places must be between 0 and 15".into());
                }
                let scale = 10_f64.powi(places as i32);
                let scaled_min = variable.min * scale;
                let scaled_max = variable.max * scale;
                if scaled_min.abs() > 9_007_199_254_740_991.0
                    || scaled_max.abs() > 9_007_199_254_740_991.0
                {
                    return Err("Range is too large for the requested decimal_places".into());
                }
                // Compare after division to avoid excluding endpoints such as 0.07
                // because their multiplication by a power of ten is inexact.
                let mut min = scaled_min.round() as i64;
                let mut max = scaled_max.round() as i64;
                if (min as f64 / scale) < variable.min {
                    min += 1;
                }
                if (max as f64 / scale) > variable.max {
                    max -= 1;
                }
                if min > max {
                    return Err("Range contains no value at the requested decimal_places".into());
                }
                Some(DecimalRange { scale, min, max })
            } else {
                None
            };
            variables.push(SimulatedVariable {
                config: variable,
                decimals,
            });
        }
        for path in &paths {
            let mut parent = path.as_str();
            while let Some((prefix, _)) = parent.rsplit_once('/') {
                if paths.contains(prefix) {
                    return Err(format!("Variable cannot be a parent folder: {prefix}"));
                }
                parent = prefix;
            }
        }
        let interval = config.simulator.create_interval()?;
        Ok(Box::new(Simulator {
            variables,
            interval,
            started_at: None,
        }))
    }
}

impl Ingestor for Simulator {
    fn next_batch(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<IngestionBatch, String>> + Send + '_>> {
        Box::pin(async move {
            self.interval.tick().await;
            let now = Instant::now();
            let elapsed = now
                .duration_since(*self.started_at.get_or_insert(now))
                .as_secs_f64();
            let mut rng = rand::rng();
            let values = self
                .variables
                .iter()
                .map(|variable| {
                    let config = &variable.config;
                    let phase = config.period_ms.filter(|ms| *ms > 0).map(|ms| {
                        let period = Duration::from_millis(ms).as_secs_f64();
                        ((elapsed % period) / period
                            + config.phase_degrees.rem_euclid(360.0) / 360.0)
                            .rem_euclid(1.0)
                    });
                    let fraction = match (config.waveform, phase) {
                        (Waveform::Sine, Some(phase)) => {
                            Some(0.5 + 0.5 * (std::f64::consts::TAU * phase).sin())
                        }
                        (Waveform::Square, Some(phase)) => {
                            Some(if phase < 0.5 { 1.0 } else { 0.0 })
                        }
                        (Waveform::Ramp, Some(phase)) => Some(phase),
                        _ => None,
                    };
                    let value = match (fraction, &variable.decimals) {
                        (None, Some(range)) => {
                            rng.random_range(range.min..=range.max) as f64 / range.scale
                        }
                        (None, None) => rng.random_range(config.min..=config.max),
                        (Some(fraction), decimals) => {
                            let value = (config.min + (config.max - config.min) * fraction)
                                .clamp(config.min, config.max);
                            match decimals {
                                Some(range) => {
                                    (value * range.scale)
                                        .round()
                                        .clamp(range.min as f64, range.max as f64)
                                        / range.scale
                                }
                                None => value,
                            }
                        }
                    };
                    VarIdValue {
                        var_id: variable.config.path.clone(),
                        value: Some(Value {
                            typed: Some(Typed::FloatValue(value)),
                        }),
                    }
                })
                .collect();
            Ok(IngestionBatch { values })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn random_config_defaults_and_rejects_invalid_intervals() {
        let config: SimulatorInstanceConfig =
            serde_json::from_value(json!({"variables":[{"path":"/test","min":0,"max":10}]}))
                .unwrap();
        assert_eq!(config.simulator.interval_ms, 1000);
        for config in [
            json!({"variables":[{"path":"/test","min":0,"max":10}],"interval_ms":0}),
            json!({"variables":[{"path":"/test","min":0,"max":10}],"interval_ms":-1}),
            json!({"variables":[{"path":"/test","min":0,"max":10}],"interval_ms":1.5}),
            json!({"variables":[{"path":"/test","min":0,"max":10}],"interval_ms":"100"}),
            json!({"variables":[{"path":"/test","min":0,"max":10}],"interval":100}),
        ] {
            assert!(
                SimulatorPlugin.create("test", &config).is_err(),
                "Accepted {config}"
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn random_uses_configured_interval_and_skips_missed_ticks() {
        let mut source = SimulatorPlugin
            .create("test", &json!({"interval_ms":250,"variables":[{"path":"/test/first","min":7,"max":7},{"path":"/test/second","min":0.07,"max":0.07,"decimal_places":2}]}))
            .unwrap();
        let first = source.next_batch().await.unwrap();
        assert_eq!(first.values.len(), 2);
        assert_eq!(first.values[0].var_id, "/test/first");
        assert_eq!(first.values[1].var_id, "/test/second");
        assert_eq!(
            first.values[1].value.as_ref().unwrap().typed,
            Some(Typed::FloatValue(0.07))
        );
        assert_eq!(
            first.values[0].value.as_ref().unwrap().typed,
            Some(Typed::FloatValue(7.0))
        );
        let start = Instant::now();
        {
            let next = source.next_batch();
            tokio::pin!(next);
            assert!(futures_util::poll!(&mut next).is_pending());
            next.await.unwrap();
        }
        assert_eq!(start.elapsed(), Duration::from_millis(250));
        tokio::time::advance(Duration::from_secs(2)).await;
        source.next_batch().await.unwrap();
        let next = source.next_batch();
        tokio::pin!(next);
        assert!(futures_util::poll!(&mut next).is_pending());
        next.await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn random_precision_preserves_bounds_and_decimal_endpoints() {
        for (min, max, places) in [
            (0.07, 0.07, 2),
            (1.1, 1.1, 2),
            (-0.075, -0.065, 2),
            (0.001, 0.029, 2),
            (-2.9, 2.9, 0),
        ] {
            let mut source = SimulatorPlugin
                .create(
                    "test",
                    &json!({"variables":[{"path":"/test","min":min,"max":max,"decimal_places":places}]}),
                )
                .unwrap();
            for _ in 0..20 {
                let batch = source.next_batch().await.unwrap();
                let Some(Typed::FloatValue(value)) = batch.values[0].value.as_ref().unwrap().typed
                else {
                    panic!("Expected float");
                };
                assert!(value >= min && value <= max);
                let scale = 10_f64.powi(places);
                assert!((value * scale - (value * scale).round()).abs() < 1e-9);
            }
        }
        for config in [
            json!({"variables":[{"path":"/test","min":0,"max":1,"decimal_places":16}]}),
            json!({"variables":[{"path":"/test","min":0,"max":1,"decimal_places":-1}]}),
            json!({"variables":[{"path":"/test","min":0,"max":1,"decimal_places":1.5}]}),
            json!({"variables":[{"path":"/test","min":0.001,"max":0.009,"decimal_places":2}]}),
            json!({"variables":[{"path":"/test","min":1e20,"max":1e21,"decimal_places":2}]}),
        ] {
            assert!(
                SimulatorPlugin.create("test", &config).is_err(),
                "Accepted {config}"
            );
        }
    }

    #[tokio::test]
    async fn rejects_empty_duplicate_invalid_and_conflicting_paths() {
        assert!(
            SimulatorPlugin
                .create("test", &json!({"variables":[]}))
                .is_err()
        );
        for path in ["relative", "/", "/a//b", "/a/", "/bad\0path", "/a", "/a/b"] {
            let config = json!({"variables":[
                {"path":"/a","min":0,"max":10},
                {"path":path,"min":0,"max":10}
            ]});
            assert!(
                SimulatorPlugin.create("test", &config).is_err(),
                "Accepted {config}"
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn periodic_waveforms_follow_period_phase_and_elapsed_time() {
        let mut source = SimulatorPlugin.create("test", &json!({
            "interval_ms":250,
            "variables":[
                {"path":"/sine","waveform":"sine","min":0,"max":10,"period_ms":1000},
                {"path":"/square","waveform":"square","min":0,"max":10,"period_ms":1000},
                {"path":"/ramp","waveform":"ramp","min":0,"max":10,"period_ms":1000},
                {"path":"/shifted","waveform":"sine","min":0,"max":10,"period_ms":1000,"phase_degrees":90},
                {"path":"/negative","waveform":"ramp","min":0,"max":10,"period_ms":2000,"phase_degrees":-90,"decimal_places":1}
            ]
        })).unwrap();
        for expected in [
            [5.0, 10.0, 0.0, 10.0, 7.5],
            [10.0, 10.0, 2.5, 5.0, 8.8],
            [5.0, 0.0, 5.0, 0.0, 0.0],
            [0.0, 0.0, 7.5, 5.0, 1.3],
            [5.0, 10.0, 0.0, 10.0, 2.5],
        ] {
            let batch = source.next_batch().await.unwrap();
            for (entry, expected) in batch.values.iter().zip(expected) {
                let Some(Typed::FloatValue(value)) = entry.value.as_ref().unwrap().typed else {
                    panic!("Expected float")
                };
                assert!(
                    (value - expected).abs() < 1e-9,
                    "{}: {value} != {expected}",
                    entry.var_id
                );
            }
        }
        tokio::time::advance(Duration::from_millis(2250)).await;
        let batch = source.next_batch().await.unwrap();
        assert_eq!(
            batch.values[2].value.as_ref().unwrap().typed,
            Some(Typed::FloatValue(2.5))
        );
    }

    #[tokio::test]
    async fn rejects_invalid_periodic_configuration() {
        for fields in [
            json!({"waveform":"sine"}),
            json!({"waveform":"square","period_ms":0}),
            json!({"waveform":"ramp","period_ms":-1}),
            json!({"waveform":"sine","period_ms":0.5}),
            json!({"waveform":"triangle","period_ms":1000}),
            json!({"waveform":"random","period_ms":1000}),
        ] {
            let mut variable = json!({"path":"/test","min":0,"max":10});
            variable
                .as_object_mut()
                .unwrap()
                .extend(fields.as_object().unwrap().clone());
            assert!(
                SimulatorPlugin
                    .create("test", &json!({"variables":[variable]}))
                    .is_err()
            );
        }
    }
}
