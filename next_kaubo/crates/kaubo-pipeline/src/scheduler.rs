use crate::cache::{ArtifactCache, CacheKey};
use crate::event::{EventHub, EventKind};
use crate::plan::{ArtifactId, NodeId, PipelinePlan, StageNode};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageError {
    pub message: String,
}

impl StageError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub trait StageAdapter {
    fn execute(
        &mut self,
        node: &StageNode,
        artifacts: &BTreeMap<ArtifactId, String>,
        events: &mut EventHub,
        cancellation: &CancellationToken,
    ) -> Result<String, StageError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionReport {
    pub executed: Vec<NodeId>,
    pub cached: Vec<NodeId>,
    pub skipped: Vec<NodeId>,
    pub artifacts: BTreeMap<ArtifactId, String>,
    pub failed: Option<(NodeId, StageError)>,
    pub cancelled: bool,
}

impl ExecutionReport {
    fn new() -> Self {
        Self {
            executed: Vec::new(),
            cached: Vec::new(),
            skipped: Vec::new(),
            artifacts: BTreeMap::new(),
            failed: None,
            cancelled: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct Scheduler {
    pub cache: ArtifactCache,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(
        &mut self,
        task_id: &str,
        source_version: u64,
        profile: &str,
        plan: &PipelinePlan,
        stage: &mut dyn StageAdapter,
        events: &mut EventHub,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionReport, String> {
        let order = plan.stable_order()?;
        let mut report = ExecutionReport::new();
        let mut completed = BTreeSet::new();

        events.publish(task_id, source_version, EventKind::TaskStarted);

        for node_id in order {
            let node = plan
                .node(node_id)
                .ok_or_else(|| format!("missing node {}", node_id.0))?;
            let deps = plan.dependencies(node_id);
            if !deps.is_subset(&completed) {
                report.skipped.push(node_id);
                continue;
            }
            if cancellation.is_cancelled() {
                report.cancelled = true;
                report.skipped.push(node_id);
                events.publish(task_id, source_version, EventKind::TaskCancelled);
                continue;
            }

            let key = CacheKey::new(
                profile,
                source_version,
                node.stage.clone(),
                node.output.clone(),
            );
            if plan.policy().use_cache {
                if let Some(hit) = self.cache.get(&key) {
                    report.artifacts.insert(node.output.clone(), hit.clone());
                    report.cached.push(node_id);
                    completed.insert(node_id);
                    events.publish(task_id, source_version, EventKind::NodeCached(node_id));
                    continue;
                }
            }

            events.publish(task_id, source_version, EventKind::NodeStarted(node_id));
            match stage.execute(node, &report.artifacts, events, cancellation) {
                Ok(artifact) => {
                    report
                        .artifacts
                        .insert(node.output.clone(), artifact.clone());
                    if plan.policy().use_cache {
                        self.cache.insert(key, artifact);
                    }
                    report.executed.push(node_id);
                    completed.insert(node_id);
                    events.publish(task_id, source_version, EventKind::NodeFinished(node_id));
                }
                Err(error) => {
                    report.failed = Some((node_id, error.clone()));
                    events.publish(task_id, source_version, EventKind::NodeFailed(node_id));
                    if plan.policy().fail_fast {
                        for rest in plan
                            .nodes()
                            .map(|node| node.id)
                            .filter(|id| !completed.contains(id) && *id != node_id)
                        {
                            report.skipped.push(rest);
                        }
                        break;
                    }
                }
            }
        }

        if !report.cancelled {
            events.publish(task_id, source_version, EventKind::TaskFinished);
        }
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{ArtifactId, PipelinePolicy, StageNode};

    fn artifact(name: &str) -> ArtifactId {
        ArtifactId::new(name)
    }

    fn node(id: usize, stage: &str, inputs: Vec<&str>, output: &str) -> StageNode {
        StageNode::new(
            NodeId(id),
            stage,
            inputs.into_iter().map(artifact).collect(),
            artifact(output),
        )
    }

    fn linear_plan() -> PipelinePlan {
        let mut plan = PipelinePlan::new(PipelinePolicy::default());
        plan.add_node(node(1, "first", vec![], "a")).unwrap();
        plan.add_node(node(2, "second", vec!["a"], "b")).unwrap();
        plan.add_edge(NodeId(1), NodeId(2)).unwrap();
        plan.request_output(artifact("b"));
        plan
    }

    #[derive(Default)]
    struct FakeStage {
        calls: Vec<NodeId>,
        fail_on: Option<NodeId>,
        cancel_after: Option<NodeId>,
    }

    impl StageAdapter for FakeStage {
        fn execute(
            &mut self,
            node: &StageNode,
            artifacts: &BTreeMap<ArtifactId, String>,
            _events: &mut EventHub,
            cancellation: &CancellationToken,
        ) -> Result<String, StageError> {
            self.calls.push(node.id);
            if self.cancel_after == Some(node.id) {
                cancellation.cancel();
            }
            if self.fail_on == Some(node.id) {
                return Err(StageError::new("boom"));
            }
            Ok(format!("{}:{}", node.stage, artifacts.len()))
        }
    }

    #[test]
    fn scheduler_executes_dependencies_in_stable_order() {
        let mut scheduler = Scheduler::new();
        let mut stage = FakeStage::default();
        let mut events = EventHub::new();
        let token = CancellationToken::new();

        let report = scheduler
            .run(
                "task",
                1,
                "check",
                &linear_plan(),
                &mut stage,
                &mut events,
                &token,
            )
            .unwrap();

        assert_eq!(stage.calls, vec![NodeId(1), NodeId(2)]);
        assert_eq!(report.executed, vec![NodeId(1), NodeId(2)]);
        assert_eq!(report.artifacts[&artifact("b")], "second:1");
        assert_eq!(events.events().first().unwrap().sequence, 0);
        assert!(matches!(
            events.events().last().unwrap().kind,
            EventKind::TaskFinished
        ));
    }

    #[test]
    fn scheduler_uses_cache_without_changing_artifact() {
        let plan = linear_plan();
        let mut scheduler = Scheduler::new();
        let mut stage = FakeStage::default();
        let mut events = EventHub::new();
        let token = CancellationToken::new();

        scheduler
            .run("task", 1, "check", &plan, &mut stage, &mut events, &token)
            .unwrap();
        let mut second_stage = FakeStage::default();
        let second = scheduler
            .run(
                "task",
                1,
                "check",
                &plan,
                &mut second_stage,
                &mut events,
                &token,
            )
            .unwrap();

        assert!(second_stage.calls.is_empty());
        assert_eq!(second.cached, vec![NodeId(1), NodeId(2)]);
        assert_eq!(second.artifacts[&artifact("b")], "second:1");
    }

    #[test]
    fn scheduler_fail_fast_skips_remaining_nodes() {
        let mut scheduler = Scheduler::new();
        let mut stage = FakeStage {
            fail_on: Some(NodeId(1)),
            ..FakeStage::default()
        };
        let mut events = EventHub::new();
        let token = CancellationToken::new();

        let report = scheduler
            .run(
                "task",
                1,
                "check",
                &linear_plan(),
                &mut stage,
                &mut events,
                &token,
            )
            .unwrap();

        assert_eq!(report.failed.as_ref().unwrap().0, NodeId(1));
        assert!(report.skipped.contains(&NodeId(2)));
        assert!(events
            .events()
            .iter()
            .any(|event| matches!(event.kind, EventKind::NodeFailed(NodeId(1)))));
    }

    #[test]
    fn scheduler_cancels_not_started_nodes() {
        let mut scheduler = Scheduler::new();
        let mut stage = FakeStage {
            cancel_after: Some(NodeId(1)),
            ..FakeStage::default()
        };
        let mut events = EventHub::new();
        let token = CancellationToken::new();

        let report = scheduler
            .run(
                "task",
                1,
                "check",
                &linear_plan(),
                &mut stage,
                &mut events,
                &token,
            )
            .unwrap();

        assert_eq!(stage.calls, vec![NodeId(1)]);
        assert!(report.cancelled);
        assert!(report.skipped.contains(&NodeId(2)));
    }
}
