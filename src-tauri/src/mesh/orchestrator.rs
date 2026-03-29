use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::capabilities::NodeCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub id: String,
    pub description: String,
    pub suggested_specialist: Option<String>,
    pub preferred_provider: Option<String>,
    pub needs_vision: bool,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeSelection {
    Local,
    Remote(String),
}

pub struct MeshOrchestrator {
    local_capabilities: NodeCapabilities,
    remote_nodes: HashMap<String, NodeCapabilities>,
}

impl MeshOrchestrator {
    pub fn new(local_caps: NodeCapabilities) -> Self {
        Self {
            local_capabilities: local_caps,
            remote_nodes: HashMap::new(),
        }
    }

    pub fn register_node(&mut self, caps: NodeCapabilities) {
        self.remote_nodes.insert(caps.node_id.clone(), caps);
    }

    pub fn remove_node(&mut self, node_id: &str) {
        self.remote_nodes.remove(node_id);
    }

    pub fn update_node_load(&mut self, node_id: &str, load: f64, active_tasks: usize) {
        if let Some(node) = self.remote_nodes.get_mut(node_id) {
            node.current_load = load;
            node.active_tasks = active_tasks;
        }
    }

    /// Score a node for a given subtask (higher is better)
    fn score_node(node: &NodeCapabilities, subtask: &SubTask) -> f64 {
        let mut score = 0.0;

        // Has required specialist? (+50)
        if let Some(ref specialist) = subtask.suggested_specialist {
            if node.installed_specialists.iter().any(|s| s == specialist) {
                score += 50.0;
            }
        }

        // Has preferred provider? (+30)
        if let Some(ref provider) = subtask.preferred_provider {
            if node.configured_providers.iter().any(|p| p == provider) {
                score += 30.0;
            }
        }

        // Low load bonus (+20 * (1 - load))
        score += 20.0 * (1.0 - node.current_load);

        // GPU bonus for vision tasks (+10)
        if subtask.needs_vision && node.has_gpu {
            score += 10.0;
        }

        // More RAM/cores = small bonus (up to +5 each)
        score += (node.ram_gb / 32.0).min(1.0) * 5.0;
        score += (node.cpu_cores as f64 / 16.0).min(1.0) * 5.0;

        score
    }

    /// Select best node for a subtask
    pub fn select_node(&self, subtask: &SubTask) -> NodeSelection {
        let local_score = Self::score_node(&self.local_capabilities, subtask);

        let mut best_remote: Option<(&str, f64)> = None;
        for (id, caps) in &self.remote_nodes {
            let score = Self::score_node(caps, subtask);
            if best_remote.is_none() || score > best_remote.unwrap().1 {
                best_remote = Some((id.as_str(), score));
            }
        }

        // Only use remote if significantly better (score > local + 20)
        if let Some((node_id, score)) = best_remote {
            if score > local_score + 20.0 {
                return NodeSelection::Remote(node_id.to_string());
            }
        }

        NodeSelection::Local
    }

    /// Plan execution: assign nodes to all subtasks
    pub fn plan_execution(&self, subtasks: &[SubTask]) -> Vec<(String, NodeSelection)> {
        subtasks
            .iter()
            .map(|st| (st.id.clone(), self.select_node(st)))
            .collect()
    }

    /// Group subtasks into parallel execution waves based on dependencies
    pub fn get_parallel_groups(&self, subtasks: &[SubTask]) -> Vec<Vec<String>> {
        let mut completed: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut groups: Vec<Vec<String>> = vec![];
        let mut remaining: Vec<&SubTask> = subtasks.iter().collect();

        while !remaining.is_empty() {
            let ready: Vec<&SubTask> = remaining
                .iter()
                .filter(|st| st.depends_on.iter().all(|dep| completed.contains(dep)))
                .cloned()
                .collect();

            if ready.is_empty() {
                // Circular dependency or orphan — take first to break the cycle
                let first = remaining.remove(0);
                groups.push(vec![first.id.clone()]);
                completed.insert(first.id.clone());
                continue;
            }

            let group_ids: Vec<String> = ready.iter().map(|st| st.id.clone()).collect();
            for id in &group_ids {
                completed.insert(id.clone());
            }
            remaining.retain(|st| !group_ids.contains(&st.id));
            groups.push(group_ids);
        }

        groups
    }

    /// Get all connected node capabilities (local + remote)
    pub fn get_all_nodes(&self) -> Vec<NodeCapabilities> {
        let mut nodes = vec![self.local_capabilities.clone()];
        nodes.extend(self.remote_nodes.values().cloned());
        nodes
    }

    /// Count of online nodes (local + remotes)
    pub fn online_node_count(&self) -> usize {
        1 + self.remote_nodes.len()
    }

    /// Get local node capabilities (mutable, for updating load)
    pub fn local_capabilities_mut(&mut self) -> &mut NodeCapabilities {
        &mut self.local_capabilities
    }
}
