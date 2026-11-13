//! Data-parallel operations on Expression DAGs powered by `spodeian-parallel`.

use crate::expr::ExprNode;
use crate::graph::ExprGraph;
use crate::id::ExprId;
use spodeian_parallel::prelude::*;

impl ExprGraph {
    /// Parallel search for all nodes matching a predicate in the graph.
    pub fn par_filter_nodes<F>(&self, predicate: F) -> Vec<(ExprId, ExprNode)>
    where
        F: Fn(&ExprNode) -> bool + Sync + Send,
    {
        let len = self.len();
        (0..len)
            .into_par_iter()
            .filter_map(|i| {
                let id = ExprId::new(i as u32);
                let node = self.get(id);
                if predicate(&node) {
                    Some((id, node))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Parallel mapping over child nodes of an expression.
    pub fn par_map_children<F>(&self, id: ExprId, map_fn: F) -> Vec<ExprId>
    where
        F: Fn(ExprId) -> ExprId + Sync + Send,
    {
        let node = self.get(id);
        let children = node.children();
        let children_vec: Vec<ExprId> = children.into_iter().collect();
        children_vec.into_par_iter().map(map_fn).collect()
    }

    /// Parallel check if any subtree of an expression matches a predicate.
    pub fn contains_matching<F>(&self, root: ExprId, predicate: &F) -> bool
    where
        F: Fn(&ExprNode) -> bool + Sync,
    {
        let root_node = self.get(root);
        if predicate(&root_node) {
            return true;
        }

        let children = root_node.children();
        let children_vec: Vec<ExprId> = children.into_iter().collect();
        children_vec
            .into_par_iter()
            .any(|child_id| self.contains_matching(child_id, predicate))
    }
}
