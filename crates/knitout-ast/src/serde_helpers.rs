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

/// Serialize/deserialize an `Either<BigDecimal, R>` with the `BigDecimal` rendered as a string.
/// `bigdecimal`'s own serde impl relies on `deserialize_any`, which non-self-describing formats
/// like postcard cannot support; going through a `String` keeps the value format-agnostic.
pub(crate) mod bigdec_either {
    use std::str::FromStr;

    use bigdecimal::BigDecimal;
    use either::Either;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<R, S>(value: &Either<BigDecimal, R>, serializer: S) -> Result<S::Ok, S::Error>
    where
        R: Serialize,
        S: Serializer,
    {
        match value {
            Either::Left(bd) => Either::<String, &R>::Left(bd.to_string()).serialize(serializer),
            Either::Right(r) => Either::<String, &R>::Right(r).serialize(serializer),
        }
    }

    pub fn deserialize<'de, R, D>(deserializer: D) -> Result<Either<BigDecimal, R>, D::Error>
    where
        R: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        Ok(match Either::<String, R>::deserialize(deserializer)? {
            Either::Left(s) => {
                Either::Left(BigDecimal::from_str(&s).map_err(serde::de::Error::custom)?)
            }
            Either::Right(r) => Either::Right(r),
        })
    }
}
