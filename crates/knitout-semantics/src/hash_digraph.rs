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
