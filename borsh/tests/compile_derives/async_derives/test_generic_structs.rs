#[cfg(feature = "hashbrown")]
use hashbrown::HashMap;

#[cfg(hash_collections)]
use core::{cmp::Eq, hash::Hash};

#[cfg(feature = "std")]
use std::collections::HashMap;

use alloc::{collections::BTreeMap, string::String};

use borsh::{BorshDeserializeAsync, BorshSerializeAsync};

#[derive(BorshSerializeAsync, BorshDeserializeAsync, Debug)]
struct TupleA<W>(W, u32);

#[derive(BorshSerializeAsync, BorshDeserializeAsync, Debug)]
struct NamedA<W> {
    a: W,
    b: u32,
}

/// `T: PartialOrd` is injected here via field bound to avoid having this restriction on
/// the struct itself
#[allow(unused)]
#[cfg(hash_collections)]
#[derive(BorshSerializeAsync)]
struct C1<T, U> {
    a: String,
    #[borsh(async_bound(serialize = "T: borsh::ser::BorshSerializeAsync + Ord,
         U: borsh::ser::BorshSerializeAsync"))]
    b: HashMap<T, U>,
}

/// `T: PartialOrd + Hash + Eq` is injected here via field bound to avoid having this restriction on
/// the struct itself
#[allow(unused)]
#[cfg(hash_collections)]
#[derive(BorshDeserializeAsync)]
struct C2<T, U> {
    a: String,
    #[borsh(
        async_bound(deserialize = "T: Ord + Hash + Eq + borsh::de::BorshDeserializeAsync,
         U: borsh::de::BorshDeserializeAsync")
    )]
    b: HashMap<T, U>,
}

/// `T: Ord` bound is required for `BorshDeserialize` derive to be successful
#[derive(BorshSerializeAsync, BorshDeserializeAsync)]
struct D<T: Ord, R> {
    a: String,
    b: BTreeMap<T, R>,
}

#[allow(unused)]
#[cfg(hash_collections)]
#[derive(BorshSerializeAsync)]
struct G<K, V, U>(
    #[borsh(skip, async_bound(serialize = "K: Sync, V: Sync"))] HashMap<K, V>,
    U,
);

#[allow(unused)]
#[cfg(hash_collections)]
#[derive(BorshDeserializeAsync)]
struct G1<K, V, U>(
    #[borsh(skip, async_bound(deserialize = "K: Send, V: Send"))] HashMap<K, V>,
    U,
);

#[allow(unused)]
#[cfg(hash_collections)]
#[derive(BorshDeserializeAsync)]
struct G2<K: Ord + Hash + Eq, R, U: Send>(HashMap<K, R>, #[borsh(skip)] U);

/// implicit derived `core::default::Default` bounds on `K` and `V` are dropped by empty bound
/// specified, as `HashMap` hash its own `Default` implementation
// looks like it's a duplicate of G1
#[allow(unused)]
#[cfg(hash_collections)]
#[derive(BorshDeserializeAsync)]
struct G3<K, V, U>(
    #[borsh(skip, async_bound(deserialize = "K: Send, V: Send"))] HashMap<K, V>,
    U,
);

#[cfg(hash_collections)]
#[derive(BorshSerializeAsync, BorshDeserializeAsync)]
struct H<K: Ord, V, U: Sync + Send> {
    x: BTreeMap<K, V>,
    #[allow(unused)]
    #[borsh(skip)]
    y: U,
}

#[allow(unused)]
trait TraitName {
    type Associated;
    fn method(&self);
}

#[allow(unused)]
#[derive(BorshSerializeAsync)]
struct ParametrizedWrongDerive<T, V>
where
    T: TraitName,
{
    #[borsh(async_bound(
        serialize = "<T as TraitName>::Associated: borsh::ser::BorshSerializeAsync"
    ))]
    field: <T as TraitName>::Associated,
    another: V,
}
