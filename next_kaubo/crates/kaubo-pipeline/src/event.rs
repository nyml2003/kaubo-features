use crate::plan::NodeId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    TaskStarted,
    NodeStarted(NodeId),
    NodeCached(NodeId),
    NodeFinished(NodeId),
    NodeFailed(NodeId),
    TaskCancelled,
    TaskFinished,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineEvent {
    pub sequence: u64,
    pub task_id: String,
    pub source_version: u64,
    pub kind: EventKind,
}

#[derive(Debug, Default)]
pub struct EventHub {
    next_sequence: u64,
    events: Vec<PipelineEvent>,
}

impl EventHub {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn publish(
        &mut self,
        task_id: impl Into<String>,
        source_version: u64,
        kind: EventKind,
    ) -> PipelineEvent {
        let event = PipelineEvent {
            sequence: self.next_sequence,
            task_id: task_id.into(),
            source_version,
            kind,
        };
        self.next_sequence += 1;
        self.events.push(event.clone());
        event
    }

    pub fn events(&self) -> &[PipelineEvent] {
        &self.events
    }

    pub fn events_for(&self, task_id: &str, source_version: u64) -> Vec<PipelineEvent> {
        self.events
            .iter()
            .filter(|event| event.task_id == task_id && event.source_version == source_version)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_hub_assigns_stable_sequences_and_filters_tasks() {
        let mut hub = EventHub::new();
        hub.publish("a", 1, EventKind::TaskStarted);
        hub.publish("b", 1, EventKind::TaskStarted);
        hub.publish("a", 2, EventKind::TaskStarted);

        assert_eq!(hub.events()[0].sequence, 0);
        assert_eq!(hub.events()[2].sequence, 2);
        assert_eq!(hub.events_for("a", 1).len(), 1);
        assert_eq!(hub.events_for("a", 2).len(), 1);
    }
}
