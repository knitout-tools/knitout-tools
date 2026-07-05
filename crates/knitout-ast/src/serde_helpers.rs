//! Serde helpers.

/// Serialize/deserialize an `IndexMap` as a sequence of `(key, value)` pairs. Needed for maps whose
/// keys are not strings (e.g. the yarn map keyed by `Carrier`), since formats like JSON only allow
/// string map keys. Insertion order is preserved, keeping output deterministic.
pub(crate) mod indexmap_seq {
    use std::hash::Hash;

    use indexmap::IndexMap;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<K, V, S>(map: &IndexMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
    where
        K: Serialize,
        V: Serialize,
        S: Serializer,
    {
        serializer.collect_seq(map.iter())
    }

    pub fn deserialize<'de, K, V, D>(deserializer: D) -> Result<IndexMap<K, V>, D::Error>
    where
        K: Deserialize<'de> + Hash + Eq,
        V: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        let pairs: Vec<(K, V)> = Vec::deserialize(deserializer)?;
        Ok(pairs.into_iter().collect())
    }
}
