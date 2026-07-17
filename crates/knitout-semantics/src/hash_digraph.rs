use std::{collections::HashMap, hash::Hash};

/// A directed graph that uses two hashmaps to store the predecessors and successors of each vertex.
#[derive(Debug, Clone)]
pub struct HashDigraph<T> {
    forward: HashMap<T, Vec<T>>,
    backward: HashMap<T, Vec<T>>,
}

impl<T> Default for HashDigraph<T> {
    fn default() -> Self {
        Self {
            forward: HashMap::new(),
            backward: HashMap::new(),
        }
    }
}

impl<T: Eq + Hash + Clone> HashDigraph<T> {
    /// Creates a new empty HashDigraph.
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_edge(&mut self, from: T, to: T) {
        self.forward
            .entry(from.clone())
            .or_default()
            .push(to.clone());
        self.backward.entry(to).or_default().push(from);
    }
}

impl<T: Eq + Hash> HashDigraph<T> {
    pub fn get_successors(&self, vertex: &T) -> Option<&[T]> {
        self.forward.get(vertex).map(|v| &v[..])
    }

    pub fn get_predecessors(&self, vertex: &T) -> Option<&[T]> {
        self.backward.get(vertex).map(|v| &v[..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_edge_records_both_directions() {
        let mut g = HashDigraph::new();
        g.add_edge(1, 2);
        g.add_edge(1, 3);
        assert_eq!(g.get_successors(&1), Some(&[2, 3][..]));
        assert_eq!(g.get_predecessors(&2), Some(&[1][..]));
        assert_eq!(g.get_predecessors(&3), Some(&[1][..]));
    }

    #[test]
    fn missing_vertices_return_none() {
        let g: HashDigraph<i32> = HashDigraph::default();
        assert_eq!(g.get_successors(&42), None);
        assert_eq!(g.get_predecessors(&42), None);
    }

    #[test]
    fn successors_and_predecessors_are_distinct() {
        let mut g = HashDigraph::new();
        g.add_edge(1, 2);
        // 1 has a successor but no predecessor; 2 has a predecessor but no successor
        assert_eq!(g.get_successors(&1), Some(&[2][..]));
        assert_eq!(g.get_predecessors(&1), None);
        assert_eq!(g.get_successors(&2), None);
        assert_eq!(g.get_predecessors(&2), Some(&[1][..]));
    }
}
