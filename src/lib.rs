// SPDX-FileCopyrightText: © 2020 Etebase Authors
// SPDX-License-Identifier: BSD-3-Clause

#![warn(clippy::all)]
#![allow(clippy::unnecessary_wraps)]

mod chunker;
mod crypto;
mod encrypted_models;
pub mod error;
pub mod fs_cache;
#[cfg(test)]
mod golden_vectors;
mod http_client;
mod online_managers;
mod service;
pub mod utils;

pub use crypto::pretty_fingerprint;

pub use http_client::Client;

pub mod http_custom_client {
    pub use crate::http_client::{ClientImplementation, Response};
}

pub use online_managers::{
    CollectionListResponse, CollectionMember, FetchOptions, ItemListResponse, IteratorListResponse,
    PrefetchOption, RemovedCollection, User, UserProfile,
};

pub use encrypted_models::{CollectionAccessLevel, ItemMetadata, SignedInvitation};

pub use service::{Account, Collection, Item};

pub mod managers {
    pub use super::service::{
        CollectionInvitationManager, CollectionManager, CollectionMemberManager, ItemManager,
    };
}

/// Helper functions for integration tests. Any items in this module is not considered part of the
/// public API and may change at any time.
#[doc(hidden)]
pub mod test_helpers {
    pub use super::{online_managers::test_reset, service::test_chunk_uids as chunk_uids};
}

pub const CURRENT_VERSION: u8 = 1;
pub const DEFAULT_SERVER_URL: &str = "https://api.etebase.com/";

pub fn init() -> error::Result<()> {
    crypto::init()
}

#[cfg(test)]
mod thread_safety {
    //! Compile-time checks: language bindings move these types between threads, and share the
    //! ones that are also `Sync` without extra locking.

    use super::*;

    fn assert_send<T: Send>() {}
    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn public_types_are_thread_safe() {
        assert_send_sync::<Client>();
        assert_send_sync::<Account>();
        assert_send_sync::<managers::CollectionManager>();
        assert_send_sync::<managers::ItemManager>();
        assert_send_sync::<managers::CollectionInvitationManager>();
        assert_send_sync::<managers::CollectionMemberManager>();
        assert_send_sync::<fs_cache::FileSystemCache>();
        assert_send_sync::<User>();
        assert_send_sync::<UserProfile>();
        assert_send_sync::<ItemMetadata>();
        assert_send_sync::<SignedInvitation>();
        assert_send_sync::<CollectionMember>();
        assert_send_sync::<RemovedCollection>();
        assert_send_sync::<IteratorListResponse<SignedInvitation>>();
        assert_send_sync::<IteratorListResponse<CollectionMember>>();
        assert_send_sync::<error::Error>();

        // The etag of these types is a `RefCell`, so they are `Send` but not `Sync`.
        assert_send::<Collection>();
        assert_send::<Item>();
        assert_send::<CollectionListResponse<Collection>>();
        assert_send::<ItemListResponse<Item>>();
        assert_send::<IteratorListResponse<Item>>();
    }
}
