use crate::graph::dynamic_prompt::DynamicPrompt;
use crate::graph::topological_sort::{CacheEntryValue, CacheView, DependencyCycleError, TopologicalSort};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub outputs: Vec<serde_json::Value>,
    pub ui: Option<serde_json::Map<String, serde_json::Value>>,
}

pub struct NullCacheView;

impl CacheView for NullCacheView {
    fn get_local(&self, _node_id: &str) -> Option<Arc<dyn CacheEntryValue>> {
        None
    }
}

impl CacheEntryValue for CacheEntry {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct ExecutionList {
    inner: TopologicalSort,
    staged_node_id: Option<String>,
    execution_cache: HashMap<String, HashMap<String, Option<Arc<dyn CacheEntryValue>>>>,
    execution_cache_listeners: HashMap<String, HashSet<String>>,
}

impl ExecutionList {
    pub fn new(dynprompt: Arc<DynamicPrompt>, cache: Arc<dyn CacheView>) -> Self {
        Self {
            inner: TopologicalSort::new(dynprompt).with_cache(cache),
            staged_node_id: None,
            execution_cache: HashMap::new(),
            execution_cache_listeners: HashMap::new(),
        }
    }

    pub fn with_null_cache(dynprompt: Arc<DynamicPrompt>) -> Self {
        Self::new(dynprompt, Arc::new(NullCacheView))
    }

    pub fn add_node(&mut self, node_id: &str, include_lazy: bool, subgraph_nodes: Option<&HashSet<String>>) {
        self.inner.add_node(node_id, include_lazy, subgraph_nodes);
    }

    pub fn add_strong_link(&mut self, from_node_id: &str, from_socket: usize, to_node_id: &str) {
        self.inner.add_strong_link(from_node_id, from_socket, to_node_id);
        self.cache_link(from_node_id, to_node_id);
    }

    pub fn cache_link(&mut self, from_node_id: &str, to_node_id: &str) {
        let to_cache = self.execution_cache.entry(to_node_id.to_string()).or_default();
        let cached_value = if self.inner.is_cached(from_node_id) {
            self.inner.cache_get_local(from_node_id)
        } else {
            None
        };
        to_cache.insert(from_node_id.to_string(), cached_value);

        self.execution_cache_listeners
            .entry(from_node_id.to_string())
            .or_default()
            .insert(to_node_id.to_string());
    }

    pub fn get_cache_entry(&self, from_node_id: &str, to_node_id: &str) -> Option<Arc<dyn CacheEntryValue>> {
        let to_cache = self.execution_cache.get(to_node_id)?;
        let value = to_cache.get(from_node_id)?;
        value.clone()
    }

    pub fn get_cache<T: CacheEntryValue + 'static>(&self, from_node_id: &str, to_node_id: &str) -> Option<&T> {
        let to_cache = self.execution_cache.get(to_node_id)?;
        let value = to_cache.get(from_node_id)?;
        value.as_ref().and_then(|arc| {
            arc.as_any().downcast_ref::<T>()
        })
    }

    pub fn cache_update(&mut self, node_id: &str, value: Arc<dyn CacheEntryValue>) {
        if let Some(listeners) = self.execution_cache_listeners.get(node_id) {
            for to_node_id in listeners {
                if let Some(to_cache) = self.execution_cache.get_mut(to_node_id) {
                    to_cache.insert(node_id.to_string(), Some(value.clone()));
                }
            }
        }
    }

    pub async fn stage_node_execution(&mut self) -> Result<Option<String>, DependencyCycleError> {
        assert!(self.staged_node_id.is_none());

        if self.inner.is_empty() {
            return Ok(None);
        }

        let mut available = self.inner.get_ready_nodes();
        while available.is_empty() && self.inner.external_blocks() > 0 {
            self.inner.unblocked_notify().notified().await;
            available = self.inner.get_ready_nodes();
        }

        if available.is_empty() {
            let cycle_nodes = self.inner.get_nodes_in_cycle();
            let _blamed_node = self.find_blamed_node(&cycle_nodes);
            return Err(DependencyCycleError);
        }

        let picked = self.ux_friendly_pick_node(&available);
        self.staged_node_id = Some(picked.to_string());
        Ok(self.staged_node_id.clone())
    }

    fn find_blamed_node(&self, cycle_nodes: &[String]) -> String {
        for node_id in cycle_nodes {
            let display_id = self.inner.dynprompt().get_display_node_id(node_id);
            if display_id != node_id {
                return display_id.to_string();
            }
        }
        cycle_nodes.first().cloned().unwrap_or_default()
    }

    fn ux_friendly_pick_node<'a>(&self, available: &[&'a str]) -> &'a str {
        for node_id in available {
            if self.is_output_node(node_id) {
                return node_id;
            }
        }

        for node_id in available {
            if let Some(blocked) = self.inner.get_blocking(node_id) {
                for blocked_id in blocked.keys() {
                    if self.is_output_node(blocked_id) {
                        return node_id;
                    }
                }
            }
        }

        for node_id in available {
            if let Some(blocked) = self.inner.get_blocking(node_id) {
                for blocked_id in blocked.keys() {
                    if let Some(blocked2) = self.inner.get_blocking(blocked_id.as_str()) {
                        for blocked_id2 in blocked2.keys() {
                            if self.is_output_node(blocked_id2) {
                                return node_id;
                            }
                        }
                    }
                }
            }
        }

        available.first().copied().unwrap_or("")
    }

    fn is_output_node(&self, _node_id: &str) -> bool {
        false
    }

    pub fn unstage_node_execution(&mut self) {
        assert!(self.staged_node_id.is_some());
        self.staged_node_id = None;
    }

    pub fn complete_node_execution(&mut self) {
        let node_id = self.staged_node_id.take().expect("No staged node to complete");
        self.inner.pop_node(&node_id);
        self.execution_cache.remove(&node_id);
        self.execution_cache_listeners.remove(&node_id);
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn staged_node_id(&self) -> Option<&str> {
        self.staged_node_id.as_deref()
    }

    pub fn dynprompt(&self) -> &DynamicPrompt {
        self.inner.dynprompt()
    }
}
