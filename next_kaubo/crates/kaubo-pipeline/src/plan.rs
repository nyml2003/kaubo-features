use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub usize);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactId(pub String);

impl ArtifactId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageNode {
    pub id: NodeId,
    pub stage: String,
    pub inputs: Vec<ArtifactId>,
    pub output: ArtifactId,
}

impl StageNode {
    pub fn new(
        id: NodeId,
        stage: impl Into<String>,
        inputs: Vec<ArtifactId>,
        output: ArtifactId,
    ) -> Self {
        Self {
            id,
            stage: stage.into(),
            inputs,
            output,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelinePolicy {
    pub fail_fast: bool,
    pub use_cache: bool,
}

impl Default for PipelinePolicy {
    fn default() -> Self {
        Self {
            fail_fast: true,
            use_cache: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelinePlan {
    nodes: BTreeMap<NodeId, StageNode>,
    edges: BTreeMap<NodeId, BTreeSet<NodeId>>,
    policy: PipelinePolicy,
    outputs: BTreeSet<ArtifactId>,
}

impl PipelinePlan {
    pub fn new(policy: PipelinePolicy) -> Self {
        Self {
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            policy,
            outputs: BTreeSet::new(),
        }
    }

    pub fn add_node(&mut self, node: StageNode) -> Result<(), String> {
        if self.nodes.contains_key(&node.id) {
            return Err(format!("duplicate node {}", node.id.0));
        }
        self.edges.entry(node.id).or_default();
        self.nodes.insert(node.id, node);
        Ok(())
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId) -> Result<(), String> {
        if !self.nodes.contains_key(&from) {
            return Err(format!("missing dependency node {}", from.0));
        }
        if !self.nodes.contains_key(&to) {
            return Err(format!("missing dependent node {}", to.0));
        }
        self.edges.entry(to).or_default().insert(from);
        Ok(())
    }

    pub fn request_output(&mut self, artifact: ArtifactId) {
        self.outputs.insert(artifact);
    }

    pub fn nodes(&self) -> impl Iterator<Item = &StageNode> {
        self.nodes.values()
    }

    pub fn node(&self, id: NodeId) -> Option<&StageNode> {
        self.nodes.get(&id)
    }

    pub fn dependencies(&self, id: NodeId) -> BTreeSet<NodeId> {
        self.edges.get(&id).cloned().unwrap_or_default()
    }

    pub fn policy(&self) -> &PipelinePolicy {
        &self.policy
    }

    pub fn outputs(&self) -> &BTreeSet<ArtifactId> {
        &self.outputs
    }

    pub fn stable_order(&self) -> Result<Vec<NodeId>, String> {
        let mut ready: BTreeSet<NodeId> = self
            .nodes
            .keys()
            .copied()
            .filter(|id| self.dependencies(*id).is_empty())
            .collect();
        let mut deps = self.edges.clone();
        let mut order = Vec::new();

        while let Some(id) = ready.pop_first() {
            order.push(id);
            for candidate in self.nodes.keys().copied() {
                if let Some(set) = deps.get_mut(&candidate) {
                    set.remove(&id);
                    if set.is_empty() && !order.contains(&candidate) {
                        ready.insert(candidate);
                    }
                }
            }
        }

        if order.len() != self.nodes.len() {
            return Err("pipeline plan contains a dependency cycle".into());
        }
        Ok(order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(name: &str) -> ArtifactId {
        ArtifactId::new(name)
    }

    #[test]
    fn stable_order_is_deterministic() {
        let mut plan = PipelinePlan::new(PipelinePolicy::default());
        plan.add_node(StageNode::new(NodeId(2), "b", vec![], artifact("b")))
            .unwrap();
        plan.add_node(StageNode::new(NodeId(1), "a", vec![], artifact("a")))
            .unwrap();
        plan.add_node(StageNode::new(NodeId(3), "c", vec![], artifact("c")))
            .unwrap();
        plan.add_edge(NodeId(1), NodeId(3)).unwrap();
        plan.add_edge(NodeId(2), NodeId(3)).unwrap();

        assert_eq!(
            plan.stable_order().unwrap(),
            vec![NodeId(1), NodeId(2), NodeId(3)]
        );
    }

    #[test]
    fn plan_rejects_bad_nodes_and_cycles() {
        let mut plan = PipelinePlan::new(PipelinePolicy::default());
        plan.add_node(StageNode::new(NodeId(1), "a", vec![], artifact("a")))
            .unwrap();
        assert!(plan
            .add_node(StageNode::new(NodeId(1), "dup", vec![], artifact("dup")))
            .is_err());
        assert!(plan.add_edge(NodeId(99), NodeId(1)).is_err());

        plan.add_node(StageNode::new(NodeId(2), "b", vec![], artifact("b")))
            .unwrap();
        plan.add_edge(NodeId(1), NodeId(2)).unwrap();
        plan.add_edge(NodeId(2), NodeId(1)).unwrap();
        assert!(plan.stable_order().unwrap_err().contains("cycle"));
    }
}
