use hashbrown::{HashMap, HashSet};
use rustc_hash::FxHasher;
use std::hash::BuildHasherDefault;

pub type FastBuildHasher = BuildHasherDefault<FxHasher>;
pub type FastHashMap<K, V> = HashMap<K, V, FastBuildHasher>;
pub type FastHashSet<T> = HashSet<T, FastBuildHasher>;
