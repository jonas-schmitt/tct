//! Strongly connected component detection for type dependency graphs.
//!
//! The type analyzer uses this helper to find recursive schema type clusters
//! before generation. Cyclic groups become analysis metadata that later passes
//! can use for boxing decisions without mixing cycle discovery into code
//! rendering.

use rustc_hash::FxHashMap;
use std::collections::HashSet;

/// Tarjan's strongly connected components algorithm for cycle detection.
pub(super) struct TarjanScc {
    graph: FxHashMap<String, HashSet<String>>,
    index: FxHashMap<String, usize>,
    lowlink: FxHashMap<String, usize>,
    on_stack: HashSet<String>,
    stack: Vec<String>,
    index_counter: usize,
    sccs: Vec<Vec<String>>,
}

impl TarjanScc {
    pub(super) fn new(graph: &FxHashMap<String, HashSet<String>>) -> Self {
        Self {
            graph: graph.clone(),
            index: FxHashMap::default(),
            lowlink: FxHashMap::default(),
            on_stack: HashSet::new(),
            stack: Vec::new(),
            index_counter: 0,
            sccs: Vec::new(),
        }
    }

    pub(super) fn find_sccs(&mut self) -> Vec<Vec<String>> {
        let nodes: Vec<String> = self.graph.keys().cloned().collect();

        for node in nodes {
            if !self.index.contains_key(&node) {
                self.strongconnect(&node);
            }
        }

        self.sccs.clone()
    }

    fn strongconnect(&mut self, v: &str) {
        // Set the depth index for v to the smallest unused index.
        self.index.insert(v.to_string(), self.index_counter);
        self.lowlink.insert(v.to_string(), self.index_counter);
        self.index_counter += 1;
        self.stack.push(v.to_string());
        self.on_stack.insert(v.to_string());

        // Consider successors of v after collection to avoid borrow conflicts.
        let successors: Vec<String> = if let Some(successors) = self.graph.get(v) {
            successors.iter().cloned().collect()
        } else {
            Vec::new()
        };

        for w in successors {
            if !self.index.contains_key(&w) {
                self.strongconnect(&w);
                let w_lowlink = self.lowlink[&w];
                let v_lowlink = self.lowlink[v];
                self.lowlink.insert(v.to_string(), v_lowlink.min(w_lowlink));
            } else if self.on_stack.contains(&w) {
                let w_index = self.index[&w];
                let v_lowlink = self.lowlink[v];
                self.lowlink.insert(v.to_string(), v_lowlink.min(w_index));
            }
        }

        if self.lowlink[v] == self.index[v] {
            let mut scc = Vec::new();
            while let Some(w) = self.stack.pop() {
                self.on_stack.remove(&w);
                scc.push(w.clone());
                if w == v {
                    break;
                }
            }
            self.sccs.push(scc);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tarjan_scc_no_cycles() {
        let mut graph = FxHashMap::default();
        let mut deps_a = HashSet::new();
        deps_a.insert("B".to_string());
        graph.insert("A".to_string(), deps_a);

        let mut deps_b = HashSet::new();
        deps_b.insert("C".to_string());
        graph.insert("B".to_string(), deps_b);

        graph.insert("C".to_string(), HashSet::new());

        let mut tarjan = TarjanScc::new(&graph);
        let sccs = tarjan.find_sccs();

        assert!(sccs.iter().all(|scc| scc.len() == 1));
    }

    #[test]
    fn tarjan_scc_simple_cycle() {
        let mut graph = FxHashMap::default();
        let mut deps_a = HashSet::new();
        deps_a.insert("B".to_string());
        graph.insert("A".to_string(), deps_a);

        let mut deps_b = HashSet::new();
        deps_b.insert("A".to_string());
        graph.insert("B".to_string(), deps_b);

        let mut tarjan = TarjanScc::new(&graph);
        let sccs = tarjan.find_sccs();

        assert!(
            sccs.iter().any(|scc| scc.len() == 2),
            "Expected to find a cycle with 2 nodes"
        );
    }

    #[test]
    fn tarjan_scc_self_loop() {
        let mut graph = FxHashMap::default();
        let mut deps_a = HashSet::new();
        deps_a.insert("A".to_string());
        graph.insert("A".to_string(), deps_a);

        let mut tarjan = TarjanScc::new(&graph);
        let sccs = tarjan.find_sccs();

        assert_eq!(sccs.len(), 1);
        assert_eq!(sccs[0].len(), 1);
        assert_eq!(sccs[0][0], "A");
    }
}
