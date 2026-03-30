pub mod namespace { include!(concat!(env!("OUT_DIR"), "/namespace.rs")); }
pub mod variable;
pub mod parser;
pub mod utils;
pub mod events;
pub mod metrics;

use std::collections::HashSet;
use super::rtdata::namespace::{event::Ev, Event};

/// Decide if an event should be sent to a client based on current subscriptions.
pub fn should_send(event: &Event, subscribed_set: &HashSet<String>, get_tree_changes: bool) -> bool {
    match &event.ev {
        Some(Ev::VarValueEv(var_id_val)) => subscribed_set.contains(&var_id_val.var_id),
        Some(Ev::TreeChangedEv(_)) => get_tree_changes,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtdata::namespace;
    use crate::rtdata::namespace::Event;

    #[test]
    fn should_send_respects_subscriptions() {
        let mut subs = HashSet::new();
        subs.insert("/a/b".to_string());
        let ev_var = Event { ev: Some(Ev::VarValueEv(namespace::VarIdValue { var_id: "/a/b".into(), value: None })) };
        let ev_other = Event { ev: Some(Ev::VarValueEv(namespace::VarIdValue { var_id: "/c".into(), value: None })) };
        let ev_tree = Event { ev: Some(Ev::TreeChangedEv(namespace::TreeChanged { folder_changed_event: vec![] })) };

        assert!(should_send(&ev_var, &subs, false));
        assert!(!should_send(&ev_other, &subs, false));
        assert!(!should_send(&ev_tree, &subs, false));
        assert!(should_send(&ev_tree, &subs, true));
    }
}
