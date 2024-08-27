//! Models for persisting data in the DB.

use borsh::{BorshDeserialize, BorshSerialize};
use clob_core::{
    api::{Dif, Request, Response},
    ClobState,
};
use paste::paste;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

macro_rules! create_model {
    ($name:ident) => {
        paste! {
            /// Wrapper type for `" $name "` so we can derive Compress and Decompress.
            // `[<$name Model>]` is special `paste!`` syntax
            #[derive(Clone, Debug, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
            pub struct [<$name Model>](pub $name);

            impl Deref for [<$name Model>] {
                type Target = $name;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl From<$name> for [<$name Model>] {
                fn from(value: $name) -> Self {
                    Self(value)
                }
            }

            impl reth_db_api::table::Compress for [<$name Model>] {
                type Compressed = Vec<u8>;

                fn compress_to_buf<B: bytes::buf::BufMut + AsMut<[u8]>>(self, dest: &mut B) {
                    let src = borsh::to_vec(&self).expect("borsh serialize works. qed.");
                    dest.put(&src[..])
                }
            }

            impl reth_db_api::table::Decompress for [<$name Model>] {
                fn decompress<B: AsRef<[u8]>>(value: B) -> Result<Self, reth_db_api::DatabaseError> {
                    borsh::from_slice(value.as_ref()).map_err(|_| reth_db_api::DatabaseError::Decode)
                }
            }
        }
    };
}

/// Ordered vec of [Dif]s.
pub type VecDif = Vec<Dif>;

create_model! { Request }
create_model! { Response }
create_model! { ClobState }
create_model! { VecDif }
