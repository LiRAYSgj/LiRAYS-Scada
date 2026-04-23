pub mod events;
pub mod metrics;
pub mod parser;
pub mod utils;
pub mod variable;

use std::collections::HashSet;

use lirays_scada_proto::namespace::v1::{Event, event::Ev};

/// Decide if an event should be sent to a client based on current subscriptions.
pub fn should_send(
    event: &Event,
    subscribed_set: &HashSet<String>,
    get_tree_changes: bool,
) -> bool {
    match &event.ev {
        Some(Ev::VarValueEv(var_id_val)) => subscribed_set.contains(&var_id_val.var_id),
        Some(Ev::TreeChangedEv(_)) => get_tree_changes,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use lirays_scada_proto::namespace::v1::{Event, TreeChanged, VarIdValue};

    use super::*;

    #[test]
    fn should_send_respects_subscriptions() {
        let mut subs = HashSet::new();
        subs.insert("/a/b".to_string());
        let ev_var = Event {
            ev: Some(Ev::VarValueEv(VarIdValue {
                var_id: "/a/b".into(),
                value: None,
            })),
        };
        let ev_other = Event {
            ev: Some(Ev::VarValueEv(VarIdValue {
                var_id: "/c".into(),
                value: None,
            })),
        };
        let ev_tree = Event {
            ev: Some(Ev::TreeChangedEv(TreeChanged {
                folder_changed_event: vec![],
                var_meta_changed_event: vec![],
            })),
        };

        assert!(should_send(&ev_var, &subs, false));
        assert!(!should_send(&ev_other, &subs, false));
        assert!(!should_send(&ev_tree, &subs, false));
        assert!(should_send(&ev_tree, &subs, true));
    }
}
