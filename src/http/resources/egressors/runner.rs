use std::{
    collections::{HashMap, HashSet},
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use lirays_scada_proto::namespace::v1::{EventBatch, VarIdValue, event::Ev};
use serde::Deserialize;
use serde_json::Value;
use tokio::{
    sync::{mpsc, watch},
    time::{Instant, Interval, MissedTickBehavior},
};
use tokio_util::sync::CancellationToken;

use crate::http::resources::plugin_instances::{InstanceState, InstanceStatus};
use crate::{
    egress::{EgressBatch, EgressPlugin, Egressor},
    rtdata::{pattern::PathPattern, variable::VariableManager},
};

const FIRST_RETRY_DELAY: Duration = Duration::from_secs(1);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);
const STOP_GRACE: Duration = Duration::from_secs(1);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EgressConfig {
    variables: Vec<String>,
    mode: EgressMode,
    target: Value,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum EgressMode {
    Subscription,
    Polling { interval_ms: u64 },
}

/// A validated instance, ready to run.
pub struct Instance {
    egressor: Box<dyn Egressor>,
    patterns: Vec<PathPattern>,
    poll_period: Option<Duration>,
}

/// Validates the engine configuration and lets the plugin validate `target`.
pub(super) fn prepare(
    plugin: &dyn EgressPlugin,
    instance_id: &str,
    config: &Value,
) -> Result<Instance, String> {
    let config: EgressConfig = serde_json::from_value(config.clone()).map_err(|e| e.to_string())?;
    if config.variables.is_empty() {
        return Err("variables must contain at least one path or pattern".into());
    }
    let patterns = config
        .variables
        .iter()
        .map(|pattern| PathPattern::parse(pattern))
        .collect::<Result<Vec<_>, _>>()?;
    let poll_period = match config.mode {
        EgressMode::Subscription => None,
        EgressMode::Polling { interval_ms } => {
            let period = Duration::from_millis(interval_ms);
            if period.is_zero() || Instant::now().checked_add(period).is_none() {
                return Err(
                    "interval_ms must be a positive, supported duration in milliseconds".into(),
                );
            }
            Some(period)
        }
    };
    Ok(Instance {
        egressor: plugin.create(instance_id, &config.target)?,
        patterns,
        poll_period,
    })
}

type Publish =
    Pin<Box<dyn Future<Output = (Box<dyn Egressor>, EgressBatch, Result<(), String>)> + Send>>;

/// Tracks which variables an instance selects and the latest unsent value of each.
struct Selection {
    patterns: Vec<PathPattern>,
    paths: HashSet<String>,
    from_events: bool,
    pending: HashMap<String, VarIdValue>,
}

impl Selection {
    /// Applies queued events. Patterns are resolved again at most once per call, and
    /// before values, so values of variables created in the same batches are kept.
    async fn apply(&mut self, variables: &VariableManager, batches: &[Arc<EventBatch>]) {
        let events = batches.iter().flat_map(|batch| &batch.events);
        let tree_changed = events.clone().any(|event| match &event.ev {
            Some(Ev::TreeChangedEv(tree)) => tree.folder_changed_event.iter().any(|folder| {
                self.patterns
                    .iter()
                    .any(|pattern| pattern.affected_by(&folder.folder_id))
            }),
            _ => false,
        });
        if tree_changed {
            self.paths = variables.resolve_patterns(&self.patterns).await;
            self.pending.retain(|path, _| self.paths.contains(path));
        }
        if self.from_events {
            for event in events {
                if let Some(Ev::VarValueEv(value)) = &event.ev
                    && self.paths.contains(&value.var_id)
                {
                    self.pending.insert(value.var_id.clone(), value.clone());
                }
            }
        }
    }

    async fn read_current(&mut self, variables: &VariableManager) {
        for value in variables.snapshot(&self.paths).await {
            self.pending.insert(value.var_id.clone(), value);
        }
    }

    fn take_batch(&mut self) -> EgressBatch {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_millis() as u64);
        EgressBatch {
            timestamp_ms,
            values: self.pending.drain().map(|(_, value)| value).collect(),
        }
    }

    /// Requeues a failed batch without overwriting newer values.
    fn restore(&mut self, batch: EgressBatch) {
        for value in batch.values {
            if self.paths.contains(&value.var_id) {
                self.pending.entry(value.var_id.clone()).or_insert(value);
            }
        }
    }
}

fn drain(
    events: &mut mpsc::Receiver<Arc<EventBatch>>,
    first: Arc<EventBatch>,
) -> Vec<Arc<EventBatch>> {
    let mut batches = vec![first];
    while let Ok(batch) = events.try_recv() {
        batches.push(batch);
    }
    batches
}

fn start_publish(mut egressor: Box<dyn Egressor>, batch: EgressBatch) -> Publish {
    Box::pin(async move {
        let result = egressor.publish(&batch).await;
        (egressor, batch, result)
    })
}

/// Collects values by subscription or polling and publishes them one batch at a time.
/// Events keep being consumed while a publish is in flight, so a slow destination
/// delays values but never loses the latest one.
pub(super) async fn run(
    instance: Instance,
    variables: Arc<VariableManager>,
    cancel: CancellationToken,
    status: watch::Sender<InstanceStatus>,
) {
    let mut events = variables.register_listener().await;
    let paths = variables.resolve_patterns(&instance.patterns).await;
    let mut selection = Selection {
        patterns: instance.patterns,
        paths,
        from_events: instance.poll_period.is_none(),
        pending: HashMap::new(),
    };
    let mut poll: Option<Interval> = instance.poll_period.map(|period| {
        let mut interval = tokio::time::interval(period);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        interval
    });
    if poll.is_none() {
        selection.read_current(&variables).await;
    }
    let mut idle = Some(instance.egressor);
    let mut in_flight: Option<Publish> = None;
    let mut retry_at: Option<Instant> = None;
    let mut retry_delay = FIRST_RETRY_DELAY;

    loop {
        if retry_at.is_none()
            && !selection.pending.is_empty()
            && let Some(egressor) = idle.take()
        {
            in_flight = Some(start_publish(egressor, selection.take_batch()));
        }
        tokio::select! {
            biased;
            _ = cancel.cancelled() => break,
            (egressor, batch, result) = async {
                match in_flight.as_mut() {
                    Some(publish) => publish.await,
                    None => std::future::pending().await,
                }
            } => {
                in_flight = None;
                idle = Some(egressor);
                match result {
                    Ok(()) => {
                        retry_delay = FIRST_RETRY_DELAY;
                        status.send_if_modified(|current| {
                            let changed = !matches!(current.state, InstanceState::Running);
                            *current = InstanceStatus { state: InstanceState::Running, last_error: None };
                            changed
                        });
                    }
                    Err(error) => {
                        log::warn!("Egressor publish failed: {error}");
                        status.send_replace(InstanceStatus {
                            state: InstanceState::Error,
                            last_error: Some(error),
                        });
                        selection.restore(batch);
                        retry_at = Some(Instant::now() + retry_delay);
                        retry_delay = (retry_delay * 2).min(MAX_RETRY_DELAY);
                    }
                }
            }
            _ = async {
                match retry_at {
                    Some(at) => tokio::time::sleep_until(at).await,
                    None => std::future::pending().await,
                }
            } => {
                retry_at = None;
            }
            _ = async {
                match poll.as_mut() {
                    Some(poll) => poll.tick().await,
                    None => std::future::pending().await,
                }
            } => {
                selection.read_current(&variables).await;
            }
            batch = events.recv() => {
                let Some(batch) = batch else {
                    status.send_replace(InstanceStatus {
                        state: InstanceState::Error,
                        last_error: Some("Namespace event stream closed".into()),
                    });
                    return;
                };
                let batches = drain(&mut events, batch);
                selection.apply(&variables, &batches).await;
            }
        }
    }

    // Values ingested right before a shutdown may still be queued: give them one last attempt.
    let finish = async {
        let egressor = match in_flight {
            Some(publish) => {
                let (egressor, batch, result) = publish.await;
                if result.is_err() {
                    selection.restore(batch);
                }
                Some(egressor)
            }
            None => idle,
        };
        let Some(mut egressor) = egressor else {
            return;
        };
        if let Ok(batch) = events.try_recv() {
            let batches = drain(&mut events, batch);
            selection.apply(&variables, &batches).await;
        }
        if !selection.pending.is_empty() {
            let _ = egressor.publish(&selection.take_batch()).await;
        }
    };
    let _ = tokio::time::timeout(STOP_GRACE, finish).await;
    status.send_replace(InstanceStatus {
        state: InstanceState::Stopped,
        last_error: None,
    });
}
