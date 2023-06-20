#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub use borsh_derive::{BorshDeserialize, BorshSchema, BorshSerialize};

pub mod de;
pub mod schema;
pub mod schema_helpers;
pub mod ser;

pub use de::BorshDeserialize;
pub use de::{from_reader, from_slice};
pub use schema::BorshSchema;
pub use schema_helpers::{try_from_slice_with_schema, try_to_vec_with_schema};
pub use ser::helpers::{to_vec, to_writer};
pub use ser::BorshSerialize;

#[cfg(all(feature = "std", feature = "external-hashcollections"))]
compile_error!(
    "feature \"std\" and feature \"external-hashcollections\" don't make sense at the same time"
);
/// A facade around all the types we need from the `std`, `core`, and `alloc`
/// crates. This avoids elaborate import wrangling having to happen in every
/// module.
#[doc(hidden)]
#[cfg(feature = "std")]
pub mod __maybestd {
    pub use std::{borrow, boxed, collections, format, io, string, vec};

    #[cfg(feature = "rc")]
    pub use std::{rc, sync};
}

#[cfg(not(feature = "std"))]
mod nostd_io;

#[doc(hidden)]
#[cfg(not(feature = "std"))]
pub mod __maybestd {
    pub use alloc::{borrow, boxed, format, string, vec};

    #[cfg(feature = "rc")]
    pub use alloc::{rc, sync};

    pub mod collections {
        pub use alloc::collections::{BTreeMap, BTreeSet, BinaryHeap, LinkedList, VecDeque};
        #[cfg(feature = "external-hashcollections")]
        pub use hashbrown::*;
    }

    pub mod io {
        pub use super::super::nostd_io::*;
    }
}
