pub mod ser {
    use crate::__maybestd::vec::Vec;
    use crate::{
        BorshSerialize,
        __maybestd::collections::{HashMap, HashSet},
    };
    use core::convert::TryFrom;
    use core::hash::BuildHasher;

    use crate::__maybestd::io::{ErrorKind, Result, Write};

    impl<K, V, H> BorshSerialize for HashMap<K, V, H>
    where
        K: BorshSerialize + PartialOrd,
        V: BorshSerialize,
        H: BuildHasher,
    {
        #[inline]
        fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
            let mut vec = self.iter().collect::<Vec<_>>();
            vec.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());
            u32::try_from(vec.len())
                .map_err(|_| ErrorKind::InvalidInput)?
                .serialize(writer)?;
            for (key, value) in vec {
                key.serialize(writer)?;
                value.serialize(writer)?;
            }
            Ok(())
        }
    }

    impl<T, H> BorshSerialize for HashSet<T, H>
    where
        T: BorshSerialize + PartialOrd,
        H: BuildHasher,
    {
        #[inline]
        fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
            let mut vec = self.iter().collect::<Vec<_>>();
            vec.sort_by(|a, b| a.partial_cmp(b).unwrap());
            u32::try_from(vec.len())
                .map_err(|_| ErrorKind::InvalidInput)?
                .serialize(writer)?;
            for item in vec {
                item.serialize(writer)?;
            }
            Ok(())
        }
    }
}

pub mod de {
    use core::hash::{BuildHasher, Hash};

    use crate::BorshDeserialize;
    use crate::__maybestd::collections::{HashMap, HashSet};
    use crate::__maybestd::io::{Read, Result};
    use crate::__maybestd::vec::Vec;

    impl<T, H> BorshDeserialize for HashSet<T, H>
    where
        T: BorshDeserialize + Eq + Hash,
        H: BuildHasher + Default,
    {
        #[inline]
        fn deserialize_reader<R: Read>(reader: &mut R) -> Result<Self> {
            let vec = <Vec<T>>::deserialize_reader(reader)?;
            Ok(vec.into_iter().collect::<HashSet<T, H>>())
        }
    }

    impl<K, V, H> BorshDeserialize for HashMap<K, V, H>
    where
        K: BorshDeserialize + Eq + Hash,
        V: BorshDeserialize,
        H: BuildHasher + Default,
    {
        #[inline]
        fn deserialize_reader<R: Read>(reader: &mut R) -> Result<Self> {
            let len = u32::deserialize_reader(reader)?;
            // TODO(16): return capacity allocation when we can safely do that.
            let mut result = HashMap::with_hasher(H::default());
            for _ in 0..len {
                let key = K::deserialize_reader(reader)?;
                let value = V::deserialize_reader(reader)?;
                result.insert(key, value);
            }
            Ok(result)
        }
    }
}
