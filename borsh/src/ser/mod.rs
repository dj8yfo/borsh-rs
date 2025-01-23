use core::{convert::TryFrom, marker::PhantomData};

use async_generic::async_generic;
#[cfg(feature = "async")]
use async_trait::async_trait;

#[cfg(feature = "async")]
use crate::async_io::AsyncWrite;
use crate::{
    __private::maybestd::{
        borrow::{Cow, ToOwned},
        boxed::Box,
        collections::{BTreeMap, BTreeSet, LinkedList, VecDeque},
        string::String,
        vec::Vec,
    },
    error::check_zst,
    io::{Error, ErrorKind, Result, Write},
};

pub(crate) mod helpers;

const FLOAT_NAN_ERR: &str = "For portability reasons we do not allow to serialize NaNs.";

/// A data-structure that can be serialized into binary format by NBOR.
///
/// ```
/// use borsh::BorshSerialize;
///
/// /// derive is only available if borsh is built with `features = ["derive"]`
/// # #[cfg(feature = "derive")]
/// #[derive(BorshSerialize)]
/// struct MyBorshSerializableStruct {
///     value: String,
/// }
///
///
/// # #[cfg(feature = "derive")]
/// let x = MyBorshSerializableStruct { value: "hello".to_owned() };
/// let mut buffer: Vec<u8> = Vec::new();
/// # #[cfg(feature = "derive")]
/// x.serialize(&mut buffer).unwrap();
/// # #[cfg(feature = "derive")]
/// let single_serialized_buffer_len = buffer.len();
///
/// # #[cfg(feature = "derive")]
/// x.serialize(&mut buffer).unwrap();
/// # #[cfg(feature = "derive")]
/// assert_eq!(buffer.len(), single_serialized_buffer_len * 2);
///
/// # #[cfg(feature = "derive")]
/// let mut buffer: Vec<u8> = vec![0; 1024 + single_serialized_buffer_len];
/// # #[cfg(feature = "derive")]
/// let mut buffer_slice_enough_for_the_data = &mut buffer[1024..1024 + single_serialized_buffer_len];
/// # #[cfg(feature = "derive")]
/// x.serialize(&mut buffer_slice_enough_for_the_data).unwrap();
/// ```
#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait(copy_sync)
)]
pub trait BorshSerialize {
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()>;

    #[inline]
    #[doc(hidden)]
    fn u8_slice(slice: &[Self]) -> Option<&[u8]>
    where
        Self: Sized,
    {
        let _ = slice;
        None
    }
}

#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait(copy_sync)
)]
impl BorshSerialize for u8 {
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        let res = writer.write_all(core::slice::from_ref(self));
        if _sync {
            res
        } else {
            res.await
        }
    }

    #[inline]
    fn u8_slice(slice: &[Self]) -> Option<&[u8]> {
        Some(slice)
    }
}

macro_rules! impl_for_integer {
    ($type: ident) => {
        #[async_generic(
            #[cfg(feature = "async")]
            #[async_trait]
            async_trait
        )]
        impl BorshSerialize for $type {
            #[inline]
            #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
            fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
                let bytes = self.to_le_bytes();
                if _sync {
                    writer.write_all(&bytes)
                } else {
                    writer.write_all(&bytes).await
                }
            }
        }
    };
}

impl_for_integer!(i8);
impl_for_integer!(i16);
impl_for_integer!(i32);
impl_for_integer!(i64);
impl_for_integer!(i128);
impl_for_integer!(u16);
impl_for_integer!(u32);
impl_for_integer!(u64);
impl_for_integer!(u128);

macro_rules! impl_for_nonzero_integer {
    ($type: ty) => {
        #[async_generic(
            #[cfg(feature = "async")]
            #[async_trait]
            async_trait
        )]
        impl BorshSerialize for $type {
            #[inline]
            #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
            fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
                if _sync {
                    BorshSerialize::serialize(&self.get(), writer)
                } else {
                    BorshSerializeAsync::serialize(&self.get(), writer).await
                }
            }
        }
    };
}

impl_for_nonzero_integer!(core::num::NonZeroI8);
impl_for_nonzero_integer!(core::num::NonZeroI16);
impl_for_nonzero_integer!(core::num::NonZeroI32);
impl_for_nonzero_integer!(core::num::NonZeroI64);
impl_for_nonzero_integer!(core::num::NonZeroI128);
impl_for_nonzero_integer!(core::num::NonZeroU8);
impl_for_nonzero_integer!(core::num::NonZeroU16);
impl_for_nonzero_integer!(core::num::NonZeroU32);
impl_for_nonzero_integer!(core::num::NonZeroU64);
impl_for_nonzero_integer!(core::num::NonZeroU128);
impl_for_nonzero_integer!(core::num::NonZeroUsize);

macro_rules! impl_for_size_integer {
    ($type:ty: $repr:ty) => {
        #[async_generic(
            #[cfg(feature = "async")]
            #[async_trait]
            async_trait
        )]
        impl BorshSerialize for $type {
            #[inline]
            #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
            fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
                if _sync {
                    BorshSerialize::serialize(&(*self as $repr), writer)
                } else {
                    BorshSerializeAsync::serialize(&(*self as $repr), writer).await
                }
            }
        }
    };
}

impl_for_size_integer!(usize: u64);
impl_for_size_integer!(isize: i64);

// Note NaNs have a portability issue. Specifically, signalling NaNs on MIPS are quiet NaNs on x86,
// and vice-versa. We disallow NaNs to avoid this issue.
macro_rules! impl_for_float {
    ($type: ident) => {
        #[async_generic(
            #[cfg(feature = "async")]
            #[async_trait]
            async_trait
        )]
        impl BorshSerialize for $type {
            #[inline]
            #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
            fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
                if self.is_nan() {
                    return Err(Error::new(ErrorKind::InvalidData, FLOAT_NAN_ERR));
                }
                let bytes = self.to_bits().to_le_bytes();
                if _sync {
                    writer.write_all(&bytes)
                } else {
                    writer.write_all(&bytes).await
                }
            }
        }
    };
}

impl_for_float!(f32);
impl_for_float!(f64);

#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait
)]
impl BorshSerialize for bool {
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        let byte = u8::from(*self);
        if _sync {
            BorshSerialize::serialize(&byte, writer)
        } else {
            BorshSerializeAsync::serialize(&byte, writer).await
        }
    }
}

#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait<T>
    where
        T: BorshSerializeAsync + Sync,
)]
impl<T> BorshSerialize for Option<T>
where
    T: BorshSerialize,
{
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            None => if _sync {
                BorshSerialize::serialize(&0u8, writer)
            } else {
                BorshSerializeAsync::serialize(&0u8, writer).await
            },
            Some(value) => if _sync {
                BorshSerialize::serialize(&1u8, writer)?;
                BorshSerialize::serialize(value, writer)
            } else {
                BorshSerializeAsync::serialize(&1u8, writer).await?;
                BorshSerializeAsync::serialize(value, writer).await
            },
        }
    }
}

#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait<T, E>
    where
        T: BorshSerializeAsync + Sync,
        E: BorshSerializeAsync + Sync,
)]
impl<T, E> BorshSerialize for core::result::Result<T, E>
where
    T: BorshSerialize,
    E: BorshSerialize,
{
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            Err(e) => if _sync {
                BorshSerialize::serialize(&0u8, writer)?;
                BorshSerialize::serialize(e, writer)
            } else {
                BorshSerializeAsync::serialize(&0u8, writer).await?;
                BorshSerializeAsync::serialize(e, writer).await
            }
            Ok(v) => if _sync {
                BorshSerialize::serialize(&1u8, writer)?;
                BorshSerialize::serialize(v, writer)
            } else {
                BorshSerializeAsync::serialize(&1u8, writer).await?;
                BorshSerializeAsync::serialize(v, writer).await
            }
        }
    }
}

#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait
)]
impl BorshSerialize for str {
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        let bytes = self.as_bytes();
        if _sync {
            BorshSerialize::serialize(bytes, writer)
        } else {
            BorshSerializeAsync::serialize(bytes, writer).await
        }
    }
}

#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait
)]
impl BorshSerialize for String {
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        let bytes = self.as_bytes();
        if _sync {
            BorshSerialize::serialize(bytes, writer)
        } else {
            BorshSerializeAsync::serialize(bytes, writer).await
        }
    }
}

/// Module is available if borsh is built with `features = ["ascii"]`.
#[cfg(feature = "ascii")]
pub mod ascii {
    //!
    //! Module defines [BorshSerialize] implementation for
    //! some types from [ascii](::ascii) crate.

    use async_generic::async_generic;
    #[cfg(feature = "async")]
    use async_trait::async_trait;

    use super::BorshSerialize;
    #[cfg(feature = "async")]
    use super::{AsyncWrite, BorshSerializeAsync};
    use crate::io::{Result, Write};

    #[async_generic(
        #[cfg(feature = "async")]
        #[async_trait]
        async_trait
    )]
    impl BorshSerialize for ascii::AsciiChar {
        #[inline]
        #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
        fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
            let byte = self.as_byte();
            if _sync {
                BorshSerialize::serialize(&byte, writer)
            } else {
                BorshSerializeAsync::serialize(&byte, writer).await
            }
        }
    }

    #[async_generic(
        #[cfg(feature = "async")]
        #[async_trait]
        async_trait
    )]
    impl BorshSerialize for ascii::AsciiStr {
        #[inline]
        #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
        fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
            let bytes = self.as_bytes();
            if _sync {
                BorshSerialize::serialize(bytes, writer)
            } else {
                BorshSerializeAsync::serialize(bytes, writer).await
            }
        }
    }

    #[async_generic(
        #[cfg(feature = "async")]
        #[async_trait]
        async_trait
    )]
    impl BorshSerialize for ascii::AsciiString {
        #[inline]
        #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
        fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
            let bytes = self.as_bytes();
            if _sync {
                BorshSerialize::serialize(bytes, writer)
            } else {
                BorshSerializeAsync::serialize(bytes, writer).await
            }
        }
    }
}

/// Helper method that is used to serialize a slice of data (without the length marker).
#[inline]
#[async_generic(
    #[cfg(feature = "async")]
    async_signature<T: BorshSerializeAsync, W: AsyncWrite>(
        data: &[T],
        writer: &mut W
    ) -> Result<()>
)]
fn serialize_slice<T: BorshSerialize, W: Write>(data: &[T], writer: &mut W) -> Result<()> {
    if let Some(u8_slice) = T::u8_slice(data) {
        if _sync {
            writer.write_all(u8_slice)
        } else {
            writer.write_all(u8_slice).await
        }?;
    } else {
        for item in data {
            if _sync {
                BorshSerialize::serialize(item, writer)
            } else {
                BorshSerializeAsync::serialize(item, writer).await
            }?;
        }
    }
    Ok(())
}

#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait<T>
    where
        T: BorshSerializeAsync + Sync,
)]
impl<T> BorshSerialize for [T]
where
    T: BorshSerialize,
{
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        let bytes = (u32::try_from(self.len()).map_err(|_| ErrorKind::InvalidData)?).to_le_bytes();
        if _sync {
            writer.write_all(&bytes)?;
            serialize_slice(self, writer)
        } else {
            writer.write_all(&bytes).await?;
            serialize_slice_async(self, writer).await
        }
    }
}

#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait<T: BorshSerializeAsync + ?Sized + Sync>
)]
impl<T: BorshSerialize + ?Sized> BorshSerialize for &T {
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        if _sync {
            BorshSerialize::serialize(*self, writer)
        } else {
            BorshSerializeAsync::serialize(*self, writer).await
        }
    }
}

#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait<T>
    where
        T: BorshSerializeAsync + ToOwned + ?Sized + Sync,
        <T as ToOwned>::Owned: Sync,
)]
impl<T> BorshSerialize for Cow<'_, T>
where
    T: BorshSerialize + ToOwned + ?Sized,
{
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        let r#ref = self.as_ref();
        if _sync {
            BorshSerialize::serialize(r#ref, writer)
        } else {
            BorshSerializeAsync::serialize(r#ref, writer).await
        }
    }
}

#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait<T>
    where
        T: BorshSerializeAsync + Sync,
)]
impl<T> BorshSerialize for Vec<T>
where
    T: BorshSerialize,
{
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        check_zst::<T>()?;

        let slice = self.as_slice();
        if _sync {
            BorshSerialize::serialize(slice, writer)
        } else {
            BorshSerializeAsync::serialize(slice, writer).await
        }
    }
}

#[cfg(feature = "bytes")]
#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait
)]
impl BorshSerialize for bytes::Bytes {
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        let bytes = self.as_ref();
        if _sync {
            BorshSerialize::serialize(bytes, writer)
        } else {
            BorshSerializeAsync::serialize(bytes, writer).await
        }
    }
}

#[cfg(feature = "bytes")]
#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait
)]
impl BorshSerialize for bytes::BytesMut {
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        let bytes = self.as_ref();
        if _sync {
            BorshSerialize::serialize(bytes, writer)
        } else {
            BorshSerializeAsync::serialize(bytes, writer).await
        }
    }
}

#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait
)]
#[cfg(feature = "bson")]
impl BorshSerialize for bson::oid::ObjectId {
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        let bytes = self.bytes();
        if _sync {
            BorshSerialize::serialize(&bytes, writer)
        } else {
            BorshSerializeAsync::serialize(&bytes, writer).await
        }
    }
}

#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait<T>
    where
        T: BorshSerializeAsync + Sync,
)]
impl<T> BorshSerialize for VecDeque<T>
where
    T: BorshSerialize,
{
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        check_zst::<T>()?;

        let bytes = (u32::try_from(self.len()).map_err(|_| ErrorKind::InvalidData)?).to_le_bytes();
        let slices = self.as_slices();
        if _sync {
            writer.write_all(&bytes)?;
            serialize_slice(slices.0, writer)?;
            serialize_slice(slices.1, writer)
        } else {
            writer.write_all(&bytes).await?;
            serialize_slice_async(slices.0, writer).await?;
            serialize_slice_async(slices.1, writer).await
        }
    }
}

#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait<T>
    where
        T: BorshSerializeAsync + Sync,
)]
impl<T> BorshSerialize for LinkedList<T>
where
    T: BorshSerialize,
{
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        check_zst::<T>()?;

        let bytes =(u32::try_from(self.len()).map_err(|_| ErrorKind::InvalidData)?).to_le_bytes();
        if _sync {
            writer.write_all(&bytes)
        } else {
            writer.write_all(&bytes).await
        }?;
        for item in self {
            if _sync {
                BorshSerialize::serialize(item, writer)
            } else {
                BorshSerializeAsync::serialize(item, writer).await
            }?;
        }
        Ok(())
    }
}

/// Module is available if borsh is built with `features = ["std"]` or `features = ["hashbrown"]`.
///
/// Module defines [BorshSerialize] implementation for
/// [HashMap](std::collections::HashMap)/[HashSet](std::collections::HashSet).
#[cfg(hash_collections)]
pub mod hashes {
    use core::{convert::TryFrom, hash::BuildHasher};

    use async_generic::async_generic;
    #[cfg(feature = "async")]
    use async_trait::async_trait;

    use crate::{
        __private::maybestd::{
            collections::{HashMap, HashSet},
            vec::Vec,
        },
        error::check_zst,
        io::{ErrorKind, Result, Write},
    };
    use super::BorshSerialize;
    #[cfg(feature = "async")]
    use super::{AsyncWrite, BorshSerializeAsync};

    #[async_generic(
        #[cfg(feature = "async")]
        #[async_trait]
        async_trait<K, V, H>
        where
            K: BorshSerializeAsync + Ord + Sync,
            V: BorshSerializeAsync + Sync,
            H: BuildHasher + Sync,
    )]
    impl<K, V, H> BorshSerialize for HashMap<K, V, H>
    where
        K: BorshSerialize + Ord,
        V: BorshSerialize,
        H: BuildHasher,
    {
        #[inline]
        #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
        fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
            check_zst::<K>()?;

            let mut vec = self.iter().collect::<Vec<_>>();
            vec.sort_by(|(a, _), (b, _)| a.cmp(b));
            let len = u32::try_from(vec.len()).map_err(|_| ErrorKind::InvalidData)?;
            if _sync {
                BorshSerialize::serialize(&len, writer)
            } else {
                BorshSerializeAsync::serialize(&len, writer).await
            }?;
            for kv in vec {
                if _sync {
                    BorshSerialize::serialize(&kv, writer)
                } else {
                    BorshSerializeAsync::serialize(&kv, writer).await
                }?;
            }
            Ok(())
        }
    }

    #[async_generic(
        #[cfg(feature = "async")]
        #[async_trait]
        async_trait<T, H>
        where
            T: BorshSerializeAsync + Ord + Sync,
            H: BuildHasher + Sync,
    )]
    impl<T, H> BorshSerialize for HashSet<T, H>
    where
        T: BorshSerialize + Ord,
        H: BuildHasher,
    {
        #[inline]
        #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
        fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
            check_zst::<T>()?;

            let mut vec = self.iter().collect::<Vec<_>>();
            vec.sort();
            let len = u32::try_from(vec.len()).map_err(|_| ErrorKind::InvalidData)?;
            if _sync {
                BorshSerialize::serialize(&len, writer)
            } else {
                BorshSerializeAsync::serialize(&len, writer).await
            }?;
            for item in vec {
                if _sync {
                    BorshSerialize::serialize(&item, writer)
                } else {
                    BorshSerializeAsync::serialize(&item, writer).await
                }?;
            }
            Ok(())
        }
    }
}

#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait<K, V>
    where
        K: BorshSerializeAsync + Sync,
        V: BorshSerializeAsync + Sync,
)]
impl<K, V> BorshSerialize for BTreeMap<K, V>
where
    K: BorshSerialize,
    V: BorshSerialize,
{
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        check_zst::<K>()?;
        // NOTE: BTreeMap iterates over the entries that are sorted by key, so the serialization
        // result will be consistent without a need to sort the entries as we do for HashMap
        // serialization.
        let len = u32::try_from(self.len()).map_err(|_| ErrorKind::InvalidData)?;
        if _sync {
            BorshSerialize::serialize(&len, writer)
        } else {
            BorshSerializeAsync::serialize(&len, writer).await
        }?;
        for (key, value) in self {
            if _sync {
                BorshSerialize::serialize(&key, writer)?;
                BorshSerialize::serialize(&value, writer)
            } else {
                BorshSerializeAsync::serialize(&key, writer).await?;
                BorshSerializeAsync::serialize(&value, writer).await
            }?;
        }
        Ok(())
    }
}

#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait<T>
    where
        T: BorshSerializeAsync + Sync,
)]
impl<T> BorshSerialize for BTreeSet<T>
where
    T: BorshSerialize,
{
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        check_zst::<T>()?;
        // NOTE: BTreeSet iterates over the items that are sorted, so the serialization result will
        // be consistent without a need to sort the entries as we do for HashSet serialization.
        let len = u32::try_from(self.len()).map_err(|_| ErrorKind::InvalidData)?;
        if _sync {
            BorshSerialize::serialize(&len, writer)
        } else {
            BorshSerializeAsync::serialize(&len, writer).await
        }?;
        for item in self {
            if _sync {
                BorshSerialize::serialize(&item, writer)
            } else {
                BorshSerializeAsync::serialize(&item, writer).await
            }?;
        }
        Ok(())
    }
}

#[cfg(feature = "std")]
#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait
)]
impl BorshSerialize for std::net::SocketAddr {
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            std::net::SocketAddr::V4(addr) => if _sync {
                BorshSerialize::serialize(&0u8, writer)?;
                BorshSerialize::serialize(addr, writer)
            } else {
                BorshSerializeAsync::serialize(&0u8, writer).await?;
                BorshSerializeAsync::serialize(addr, writer).await
            }
            std::net::SocketAddr::V6(addr) => if _sync {
                BorshSerialize::serialize(&1u8, writer)?;
                BorshSerialize::serialize(addr, writer)
            } else {
                BorshSerializeAsync::serialize(&1u8, writer).await?;
                BorshSerializeAsync::serialize(addr, writer).await
            }
        }
    }
}

#[cfg(feature = "std")]
#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait
)]
impl BorshSerialize for std::net::SocketAddrV4 {
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        if _sync {
            BorshSerialize::serialize(self.ip(), writer)?;
            BorshSerialize::serialize(&self.port(), writer)
        } else {
            BorshSerializeAsync::serialize(self.ip(), writer).await?;
            BorshSerializeAsync::serialize(&self.port(), writer).await
        }
    }
}

#[cfg(feature = "std")]
#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait
)]
impl BorshSerialize for std::net::SocketAddrV6 {
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        if _sync {
            BorshSerialize::serialize(self.ip(), writer)?;
            BorshSerialize::serialize(&self.port(), writer)
        } else {
            BorshSerializeAsync::serialize(self.ip(), writer).await?;
            BorshSerializeAsync::serialize(&self.port(), writer).await
        }
    }
}

#[cfg(feature = "std")]
#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait
)]
impl BorshSerialize for std::net::Ipv4Addr {
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        if _sync {
            writer.write_all(&self.octets())
        } else {
            writer.write_all(&self.octets()).await
        }
    }
}

#[cfg(feature = "std")]
#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait
)]
impl BorshSerialize for std::net::Ipv6Addr {
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        if _sync {
            writer.write_all(&self.octets())
        } else {
            writer.write_all(&self.octets()).await
        }
    }
}

#[cfg(feature = "std")]
#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait
)]
impl BorshSerialize for std::net::IpAddr {
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            std::net::IpAddr::V4(ipv4) => if _sync {
                writer.write_all(&0u8.to_le_bytes())?;
                BorshSerialize::serialize(ipv4, writer)
            } else {
                writer.write_all(&0u8.to_le_bytes()).await?;
                BorshSerializeAsync::serialize(ipv4, writer).await
            }
            std::net::IpAddr::V6(ipv6) => if _sync {
                writer.write_all(&1u8.to_le_bytes())?;
                BorshSerialize::serialize(ipv6, writer)
            } else {
                writer.write_all(&1u8.to_le_bytes()).await?;
                BorshSerializeAsync::serialize(ipv6, writer).await
            }
        }
    }
}

#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait<T>
    where
        T: BorshSerializeAsync + ?Sized + Sync,
)]
impl<T: BorshSerialize + ?Sized> BorshSerialize for Box<T> {
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        let r#ref = self.as_ref();
        if _sync {
            BorshSerialize::serialize(r#ref, writer)
        } else {
            BorshSerializeAsync::serialize(r#ref, writer).await
        }
    }
}

#[async_generic(
    #[cfg(feature = "async")]
    #[async_trait]
    async_trait<T, const N: usize>
    where
        T: BorshSerializeAsync + Sync,
)]
impl<T, const N: usize> BorshSerialize for [T; N]
where
    T: BorshSerialize,
{
    #[inline]
    #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        if N == 0 {
            Ok(())
        } else if let Some(u8_slice) = T::u8_slice(self) {
            if _sync {
                writer.write_all(u8_slice)
            } else {
                writer.write_all(u8_slice).await
            }
        } else {
            for el in self.iter() {
                if _sync {
                    BorshSerialize::serialize(el, writer)
                } else {
                    BorshSerializeAsync::serialize(el, writer).await
                }?;
            }
            Ok(())
        }
    }
}

macro_rules! impl_tuple {
    (@unit $name:ty) => {
        #[async_generic(
            #[cfg(feature = "async")]
            #[async_trait]
            async_trait
        )]
        impl BorshSerialize for $name {
            #[inline]
            #[async_generic(async_signature<W: AsyncWrite>(&self, _writer: &mut W) -> Result<()>)]
            fn serialize<W: Write>(&self, _writer: &mut W) -> Result<()> {
                Ok(())
            }
        }
    };

    ($($idx:tt $name:ident)+) => {
        #[async_generic(
            #[cfg(feature = "async")]
            #[async_trait]
            async_trait<$($name),+>
            where
                $($name: BorshSerializeAsync + Sync + Send,)+
        )]
        impl<$($name),+> BorshSerialize for ($($name,)+)
        where
            $($name: BorshSerialize,)+
        {
            #[inline]
            #[async_generic(async_signature<W: AsyncWrite>(&self, writer: &mut W) -> Result<()>)]
            fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
                if _sync {
                    $(BorshSerialize::serialize(&self.$idx, writer)?;)+
                } else {
                    $(BorshSerializeAsync::serialize(&self.$idx, writer).await?;)+
                }
                Ok(())
            }
        }
    };
}

impl_tuple!(@unit ());
impl_tuple!(@unit core::ops::RangeFull);

impl_tuple!(0 T0);
impl_tuple!(0 T0 1 T1);
impl_tuple!(0 T0 1 T1 2 T2);
impl_tuple!(0 T0 1 T1 2 T2 3 T3);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16 17 T17);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16 17 T17 18 T18);
impl_tuple!(0 T0 1 T1 2 T2 3 T3 4 T4 5 T5 6 T6 7 T7 8 T8 9 T9 10 T10 11 T11 12 T12 13 T13 14 T14 15 T15 16 T16 17 T17 18 T18 19 T19);

macro_rules! impl_range {
    ($type:ident, $this:ident, $($field:expr),*) => {
        impl<T: BorshSerialize> BorshSerialize for core::ops::$type<T> {
            #[inline]
            fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
                let $this = self;
                $( $field.serialize(writer)?; )*
                Ok(())
            }
        }
    };
}

impl_range!(Range, this, &this.start, &this.end);
impl_range!(RangeInclusive, this, this.start(), this.end());
impl_range!(RangeFrom, this, &this.start);
impl_range!(RangeTo, this, &this.end);
impl_range!(RangeToInclusive, this, &this.end);

/// Module is available if borsh is built with `features = ["rc"]`.
#[cfg(feature = "rc")]
pub mod rc {
    //!
    //! Module defines [BorshSerialize] implementation for
    //! [alloc::rc::Rc](std::rc::Rc) and [alloc::sync::Arc](std::sync::Arc).
    use crate::{
        __private::maybestd::{rc::Rc, sync::Arc},
        io::{Result, Write},
        BorshSerialize,
    };

    /// This impl requires the [`"rc"`] Cargo feature of borsh.
    ///
    /// Serializing a data structure containing `Rc` will serialize a copy of
    /// the contents of the `Rc` each time the `Rc` is referenced within the
    /// data structure. Serialization will not attempt to deduplicate these
    /// repeated data.
    impl<T: BorshSerialize + ?Sized> BorshSerialize for Rc<T> {
        fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
            (**self).serialize(writer)
        }
    }

    /// This impl requires the [`"rc"`] Cargo feature of borsh.
    ///
    /// Serializing a data structure containing `Arc` will serialize a copy of
    /// the contents of the `Arc` each time the `Arc` is referenced within the
    /// data structure. Serialization will not attempt to deduplicate these
    /// repeated data.
    impl<T: BorshSerialize + ?Sized> BorshSerialize for Arc<T> {
        fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
            (**self).serialize(writer)
        }
    }
}

impl<T: ?Sized> BorshSerialize for PhantomData<T> {
    fn serialize<W: Write>(&self, _: &mut W) -> Result<()> {
        Ok(())
    }
}

impl<T> BorshSerialize for core::cell::Cell<T>
where
    T: BorshSerialize + Copy,
{
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        <T as BorshSerialize>::serialize(&self.get(), writer)
    }
}

impl<T> BorshSerialize for core::cell::RefCell<T>
where
    T: BorshSerialize + Sized,
{
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        match self.try_borrow() {
            Ok(ref value) => value.serialize(writer),
            Err(_) => Err(Error::new(ErrorKind::Other, "already mutably borrowed")),
        }
    }
}
