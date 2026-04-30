use crate::graph::dynamic_prompt::DynamicPrompt;
use crate::graph::edge::{is_link, parse_link, StrongLink};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Debug, thiserror::Error)]
#[error("Dependency cycle detected")]
pub struct DependencyCycleError;

#[derive(Debug, thiserror::Error)]
#[error("Node input error: {message}")]
pub struct NodeInputError {
    pub message: String,
}

pub trait CacheView: Send + Sync {
    fn get_local(&self, node_id: &str) -> Option<Arc<dyn CacheEntryValue>>;
}

pub trait CacheEntryValue: Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
}

struct SharedState {
    block_count: HashMap<String, usize>,
    external_blocks: usize,
}

pub struct UnblockGuard {
    shared: Arc<std::sync::Mutex<SharedState>>,
    node_id: String,
    notify: Arc<Notify>,
    armed: bool,
}

impl UnblockGuard {
    pub fn unblock(mut self) {
        self.do_unblock();
    }

    fn do_unblock(&mut self) {
        if self.armed {
            self.armed = false;
            let mut state = self.shared.lock().unwrap();
            if let Some(count) = state.block_count.get_mut(&self.node_id) {
                *count = count.saturating_sub(1);
            }
            state.external_blocks = state.external_blocks.saturating_sub(1);
            drop(state);
            self.notify.notify_waiters();
        }
    }
}

impl Drop for UnblockGuard {
    fn drop(&mut self) {
        self.do_unblock();
    }
}

pub struct TopologicalSort {
    dynprompt: Arc<DynamicPrompt>,
    pending_nodes: HashSet<String>,
    shared: Arc<std::sync::Mutex<SharedState>>,
    blocking: HashMap<String, HashMap<String, HashSet<usize>>>,
    unblocked_notify: Arc<Notify>,
    cache: Option<Arc<dyn CacheView>>,
}

impl TopologicalSort {
    pub fn new(dynprompt: Arc<DynamicPrompt>) -> Self {
        Self {
            dynprompt,
            pending_nodes: HashSet::new(),
            shared: Arc::new(std::sync::Mutex::new(SharedState {
                block_count: HashMap::new(),
                external_blocks: 0,
            })),
            blocking: HashMap::new(),
            unblocked_notify: Arc::new(Notify::new()),
            cache: None,
        }
    }

    pub fn with_cache(mut self, cache: Arc<dyn CacheView>) -> Self {
        self.cache = Some(cache);
        self
    }

    pub fn dynprompt(&self) -> &DynamicPrompt {
        &self.dynprompt
    }

    pub fn add_node(&mut self, node_id: &str, include_lazy: bool, subgraph_nodes: Option<&HashSet<String>>) {
        let mut node_ids = vec![node_id.to_string()];
        let mut links: Vec<StrongLink> = Vec::new();

        while let Some(uid) = node_ids.pop() {
            if self.pending_nodes.contains(&uid) {
                continue;
            }

            self.pending_nodes.insert(uid.clone());
            {
                let mut state = self.shared.lock().unwrap();
                state.block_count.insert(uid.clone(), 0);
            }
            self.blocking.insert(uid.clone(), HashMap::new());

            let node = match self.dynprompt.get_node(&uid) {
                Ok(n) => n,
                Err(_) => continue,
            };

            for (input_name, value) in &node.inputs {
                if !is_link(value) {
                    continue;
                }
                let link = match parse_link(value) {
                    Some(l) => l,
                    None => continue,
                };
                if let Some(subgraph) = subgraph_nodes {
                    if !subgraph.contains(&link.from_node) {
                        continue;
                    }
                }
                let is_lazy = self.is_lazy_input(input_name);
                if !include_lazy && is_lazy {
                    continue;
                }
                if !self.is_cached(&link.from_node) {
                    node_ids.push(link.from_node.clone());
                }
                links.push(StrongLink::new(&link.from_node, link.from_socket, &uid));
            }
        }

        for link in links {
            self.add_strong_link(&link.from_node, link.from_socket, &link.to_node);
        }
    }

    pub fn add_strong_link(&mut self, from_node_id: &str, from_socket: usize, to_node_id: &str) {
        if !self.is_cached(from_node_id) {
            if !self.pending_nodes.contains(from_node_id) {
                self.add_node(from_node_id, false, None);
            }
            let from_blocking = self.blocking.entry(from_node_id.to_string()).or_default();
            let mut state = self.shared.lock().unwrap();
            if !from_blocking.contains_key(to_node_id) {
                from_blocking.insert(to_node_id.to_string(), HashSet::new());
                *state.block_count.entry(to_node_id.to_string()).or_insert(0) += 1;
            }
            drop(state);
            from_blocking
                .get_mut(to_node_id)
                .unwrap()
                .insert(from_socket);
        }
    }

    pub fn make_input_strong_link(&mut self, to_node_id: &str, to_input: &str) -> Result<(), NodeInputError> {
        let node = self.dynprompt.get_node(to_node_id).map_err(|_| NodeInputError {
            message: format!("Node {} not found", to_node_id),
        })?;
        let value = node.inputs.get(to_input).ok_or_else(|| NodeInputError {
            message: format!(
                "Node {} says it needs input {}, but there is no input to that node",
                to_node_id, to_input
            ),
        })?;
        if !is_link(value) {
            return Err(NodeInputError {
                message: format!(
                    "Node {} says it needs input {}, but that value is a constant",
                    to_node_id, to_input
                ),
            });
        }
        let link = parse_link(value).ok_or_else(|| NodeInputError {
            message: format!("Invalid link format for input {} of node {}", to_input, to_node_id),
        })?;
        self.add_strong_link(&link.from_node, link.from_socket, to_node_id);
        Ok(())
    }

    pub fn add_external_block(&mut self, node_id: &str) -> Result<UnblockGuard, String> {
        let state = self.shared.lock().unwrap();
        if !state.block_count.contains_key(node_id) {
            return Err(format!(
                "Can't add external block to node {} that isn't pending",
                node_id
            ));
        }
        drop(state);

        {
            let mut state = self.shared.lock().unwrap();
            state.external_blocks += 1;
            *state.block_count.get_mut(node_id).unwrap() += 1;
        }

        Ok(UnblockGuard {
            shared: self.shared.clone(),
            node_id: node_id.to_string(),
            notify: self.unblocked_notify.clone(),
            armed: true,
        })
    }

    fn is_lazy_input(&self, _input_name: &str) -> bool {
        false
    }

    pub fn is_cached(&self, node_id: &str) -> bool {
        if let Some(cache) = &self.cache {
            cache.get_local(node_id).is_some()
        } else {
            false
        }
    }

    pub fn cache_get_local(&self, node_id: &str) -> Option<Arc<dyn CacheEntryValue>> {
        self.cache.as_ref().and_then(|c| c.get_local(node_id))
    }

    pub fn get_ready_nodes(&self) -> Vec<&str> {
        let state = self.shared.lock().unwrap();
        self.pending_nodes
            .iter()
            .filter(|id| state.block_count.get(*id).copied().unwrap_or(0) == 0)
            .map(|s| s.as_str())
            .collect()
    }

    pub fn pop_node(&mut self, node_id: &str) {
        self.pending_nodes.remove(node_id);
        if let Some(blocked_nodes) = self.blocking.remove(node_id) {
            let mut state = self.shared.lock().unwrap();
            for blocked_id in blocked_nodes.keys() {
                if let Some(count) = state.block_count.get_mut(blocked_id) {
                    *count = count.saturating_sub(1);
                }
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.pending_nodes.is_empty()
    }

    pub fn pending_count(&self) -> usize {
        self.pending_nodes.len()
    }

    pub fn unblocked_notify(&self) -> &Notify {
        &self.unblocked_notify
    }

    pub fn external_blocks(&self) -> usize {
        self.shared.lock().unwrap().external_blocks
    }

    pub fn get_blocking(&self, node_id: &str) -> Option<&HashMap<String, HashSet<usize>>> {
        self.blocking.get(node_id)
    }

    pub fn get_nodes_in_cycle(&self) -> Vec<String> {
        let mut blocked_by: HashMap<String, HashMap<String, bool>> = self
            .pending_nodes
            .iter()
            .map(|id| (id.clone(), HashMap::new()))
            .collect();

        for (from_id, blocked_nodes) in &self.blocking {
            for to_id in blocked_nodes.keys() {
                if let Some(entry) = blocked_by.get_mut(to_id) {
                    entry.insert(from_id.clone(), true);
                }
            }
        }

        loop {
            let to_remove: Vec<String> = blocked_by
                .iter()
                .filter(|(_, blockers)| blockers.is_empty())
                .map(|(id, _)| id.clone())
                .collect();

            if to_remove.is_empty() {
                break;
            }

            for remove_id in &to_remove {
                blocked_by.remove(remove_id);
                for (_, blockers) in blocked_by.iter_mut() {
                    blockers.remove(remove_id);
                }
            }
        }

        blocked_by.into_keys().collect()
    }
}
