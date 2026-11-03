// SPDX-FileCopyrightText: © 2020 Etebase Authors
// SPDX-License-Identifier: LGPL-2.1-only

//! Golden vectors recorded with the sodiumoxide-based build (libsodium 1.0.18 via libsodium-sys).
//!
//! Etebase servers store data produced by these primitives, so any replacement crypto backend must
//! reproduce every value here byte for byte. Never re-record these values to make a test pass.
//!
//! Randomized operations (nonces, chunk shuffling, key generation) cannot be recorded on the
//! encryption side, so they are covered by decrypting recorded ciphertexts instead.

use std::convert::TryInto;

use crate::crypto::{derive_key, BoxCryptoManager, CryptoMac, CryptoManager, LoginCryptoManager};
use crate::encrypted_models::{AccountCryptoManager, EncryptedCollection};
use crate::utils::{
    buffer_pad, buffer_pad_fixed, buffer_pad_small, buffer_unpad, buffer_unpad_fixed, from_base64,
    get_padding, randombytes_deterministic,
};

const CLEARTEXT: &[u8] = b"Some cleartext";

/// `[start, start + 1, ..., start + 31]`
fn seq32(start: u8) -> [u8; 32] {
    let mut ret = [0; 32];
    for (i, x) in ret.iter_mut().enumerate() {
        *x = start.wrapping_add(i as u8);
    }
    ret
}

fn b64(s: &str) -> Vec<u8> {
    from_base64(s).unwrap()
}

fn main_crypto_manager() -> CryptoManager {
    CryptoManager::new(&seq32(0), b"Main    ", 1).unwrap()
}

#[test]
fn argon2id_derive_key() {
    crate::init().unwrap();

    let salt: [u8; 16] = (1..=16).collect::<Vec<u8>>().try_into().unwrap();
    assert_eq!(
        derive_key(&salt, "correct horse battery staple ✓").unwrap()[..],
        b64("aojHfdzphk5UVWxnqNBieL5hPoJpR3PJUtL-zBrcnMQ")[..]
    );
    assert_eq!(
        derive_key(&[0; 16], "").unwrap()[..],
        b64("Yyby0ZaZRX6BX1TqA7WYxJ8CW_wpMmzxnahN8czq8Vw")[..]
    );
}

/// `crypto_kdf` subkeys for every context in use, observed through each derived key.
#[test]
fn kdf_contexts() {
    crate::init().unwrap();

    // (context, asym_key_seed, derive_subkey, calculate_mac, deterministic_encrypt with AD)
    let vectors: [(&[u8; 8], &str, &str, &str, &str); 5] = [
        (
            b"Main    ",
            "7saDy09w4PIzbhA2xJFOBD0-fXmzwd0tDnFBKTrlxFw",
            "hx9b1655MU1hlEqE8X-ov2CuEeJ0RogEpEFObcjLg_I",
            "dfOXuD7kOegTi946_3QM6idf6LBjPafIYlJYYNMbCyQ",
            "dfOXuD7kOegTi946_3QM6idf6LBjPafI6elx5CDWRoKjyA5TbB2PpW6rKQ",
        ),
        (
            b"Stor    ",
            "B9S7Ww_0-fGVwAvS3ohHhBFwQG37wHT0KfxiZvQzGsw",
            "cJjK1ZXfWZzVpOQhZ946YG7XTlULCCzPO8hKput2-AY",
            "6VV61FqkoCy7yy3eXgsWmWIfZJsbdrDx5g8gJrTuhU4",
            "6VV61FqkoCy7yy3eXgsWmWIfZJsbdrDxKS26m5PTlL3vl33jJy3pHA6N-g",
        ),
        (
            b"Acct    ",
            "xesiZhFrYN-cn6vC94E0C2capTSfX6_CQHZrvfogqjo",
            "J_xvOYP87zhdk6WyA3bLMk14hjbrxDIjDzKu9849ucY",
            "hs2g7WvcEEau3PzlkLdifGOA_r8jz4ADLYPwrlX2lL4",
            "hs2g7WvcEEau3PzlkLdifGOA_r8jz4ADf7WKA6Y_ERnkSeUzzeuFB60Vhw",
        ),
        (
            b"Col     ",
            "OFqRVxtV-79jD0YJlrtGHJOanypkXOBU2fhe9Wl1IjQ",
            "_YzABrEZFUQ9eTKWTKYSYtsnG5mkUSzDUWOpx7X4zpI",
            "6fF34AmSRVF3ynqo_K2ZyruiJu_wF4dUb5AVxfU3XxA",
            "6fF34AmSRVF3ynqo_K2ZyruiJu_wF4dUBAiiBbWXiHy5VNPEswVu1QJ4ug",
        ),
        (
            b"ColItem ",
            "VXoCOuQp_4H5HYiVXEJAz1M2IeH5e9_T2sPY1LbeTP0",
            "xvvJLjhemWKXF8ub7OrNBGWMZ16QLbdTTj_5rffNMts",
            "VZJIXxcmuX6vmQXOcfVVp490mrDhHSH62089lb6rO0g",
            "VZJIXxcmuX6vmQXOcfVVp490mrDhHSH6Ap9OipjgtDmnnXRiBM0zcj8gfw",
        ),
    ];

    for (context, asym_key_seed, subkey, mac, deterministic) in vectors {
        let cm = CryptoManager::new(&seq32(0), context, 1).unwrap();
        assert_eq!(cm.asym_key_seed[..], b64(asym_key_seed)[..]);
        assert_eq!(
            cm.derive_subkey(b"0123456789abcdef0123456789abcdef")
                .unwrap()[..],
            b64(subkey)[..]
        );
        assert_eq!(cm.calculate_mac(b"msg").unwrap()[..], b64(mac)[..]);
        let cipher = b64(deterministic);
        assert_eq!(
            cm.deterministic_encrypt(b"msg", Some(b"ad")).unwrap(),
            cipher
        );
        assert_eq!(
            cm.deterministic_decrypt(&cipher, Some(b"ad")).unwrap(),
            b"msg"
        );
    }
}

/// `crypto_aead_xchacha20poly1305_ietf` with random nonces: the layout is `nonce ‖ ciphertext ‖ tag`.
#[test]
fn xchacha20poly1305_decrypt() {
    crate::init().unwrap();
    let cm = main_crypto_manager();

    let cipher = b64("xQfHPGwCOTLFQjdmHvApiwjxLmv9pPkYjmAdB2iXj0WZUdUzk7BuQc8wpEj4t9uKV7CEt_op");
    assert_eq!(cm.decrypt(&cipher, None).unwrap(), CLEARTEXT);
    assert!(cm.decrypt(&cipher, Some(b"ad")).is_err());

    let cipher = b64("ZVsVX9cc3KGcwWsNqWInGIVTYZ2sxPYzFM8Mh2YibaVb8KKnkBuOhZWy4l8G74Qm5Iozj84f");
    assert_eq!(cm.decrypt(&cipher, Some(b"ad")).unwrap(), CLEARTEXT);
    assert!(cm.decrypt(&cipher, None).is_err());

    // Detached: the tag is returned separately and the cipher is `nonce ‖ ciphertext`.
    let tag: [u8; 16] = b64("1_6MRR_6mcYrdQ9A7ZMeIg").try_into().unwrap();
    let cipher = b64("q5RoLAM-ndT5kI4fqq58Y_GtfcuDOMIJfoyeooug-hC-FJu01gQ");
    assert_eq!(
        cm.decrypt_detached(&cipher, &tag, Some(b"ad")).unwrap(),
        CLEARTEXT
    );
    assert!(cm.verify(&cipher, &tag, Some(b"ad")).unwrap());
    assert!(cm.decrypt_detached(&cipher, &tag, None).is_err());
}

/// Keyed and unkeyed `crypto_generichash` (BLAKE2b) with a 32-byte output.
#[test]
fn blake2b_generichash() {
    crate::init().unwrap();

    let long: Vec<u8> = (0..1000u32).map(|x| (x % 251) as u8).collect();
    let mut key = [0; 64];
    key[..32].copy_from_slice(&seq32(0));
    key[32..].copy_from_slice(&seq32(32));

    let hash = |key: Option<&[u8]>, msg: &[u8]| {
        let mut mac = CryptoMac::new(key).unwrap();
        mac.update(msg).unwrap();
        mac.finalize().unwrap()
    };

    assert_eq!(
        hash(None, b""),
        b64("DldRwCblQ7Loqy6wYJnaodHl30d3j3eH-qtFzfEv46g")
    );
    assert_eq!(
        hash(None, &long),
        b64("s3LQYI9yDIw91B6cjuyxAUO0Gr5SC2FmB-dUv3nAgzE")
    );
    assert_eq!(
        hash(Some(&key[..16]), &long),
        b64("_K41wqOfU3KvCC2SI3Hcmof7tKcpcun5HMtnVUPnHo8")
    );
    assert_eq!(
        hash(Some(&key), &long),
        b64("wxWVB-pfhw7BHwshkkcuPu1Dzqtx8Jx4L_1SxkYB6rE")
    );
}

/// `crypto_box` (curve25519xsalsa20poly1305): seeded keypairs and decryption of a recorded box.
#[test]
fn crypto_box() {
    crate::init().unwrap();

    let a = BoxCryptoManager::keygen(Some(&seq32(1))).unwrap();
    let b = BoxCryptoManager::keygen(Some(&seq32(2))).unwrap();
    assert_eq!(
        a.pubkey(),
        b64("SjgH0GTQdxgcwHCYnnaJHSDcpVWVSNwsd8GlAnOIKzg")
    );
    assert_eq!(
        a.privkey(),
        b64("d3iPGgzqABomMdrl0F29BiAI1bMPULnim-sqeCIokAQ")
    );
    assert_eq!(
        b.pubkey(),
        b64("2AKgsNn2QueLyPryEL9zjXrpGjE7A__CwRyEudgB0Hg")
    );
    assert_eq!(
        b.privkey(),
        b64("Lhqfw9OX1WHNwmWMb11ynInf-e10mqpitT_VbV4f-P8")
    );

    // The public key must be recoverable from the private key alone (used when restoring accounts).
    let restored = BoxCryptoManager::from_privkey(a.privkey().try_into().unwrap()).unwrap();
    assert_eq!(restored.pubkey(), a.pubkey());

    // Boxed from a to b.
    let cipher = b64("YFiq45RyYgSpHmDw_OkQPM2wPKvg8csd6wcoWq3TK6LpnCMEqpOAn-cIL8U7eLnUAO84mxip");
    assert_eq!(
        b.decrypt(&cipher, a.pubkey().try_into().unwrap()).unwrap(),
        CLEARTEXT
    );
    assert!(b.decrypt(&cipher, b.pubkey().try_into().unwrap()).is_err());
}

/// `crypto_sign` (ed25519): seeded keypair and deterministic detached signature.
#[test]
fn crypto_sign() {
    crate::init().unwrap();

    let login = LoginCryptoManager::keygen(&seq32(3)).unwrap();
    assert_eq!(
        login.pubkey(),
        b64("SB7edy2hV4XJ5ZoIYgG4w_nDBxKeP8bX80qk3-1YFOU")
    );
    assert_eq!(
        login.sign_detached(b"Some message").unwrap(),
        b64("jasGSja5XIY7TrWhMZG9k3fPSAFdAtelcHAGsJqRI8vjy7QmDt1vArktqtuQVodMGbTJWDlltopW7vcTpiMfCw")
    );
}

#[test]
fn padme_lengths() {
    // Note that lengths that are already a valid padme size map to themselves; `buffer_pad` then
    // adds a whole extra block for them.
    let vectors = [
        (0, 1024),
        (1, 1024),
        (1023, 1024),
        (1024, 2048),
        (16383, 16384),
        (16384, 16384),
        (16385, 17408),
        (65536, 65536),
        (100000, 100352),
        (2343242, 2359296),
        (1 << 30, 1 << 30),
    ];
    for (length, padded) in vectors {
        assert_eq!(get_padding(length), padded, "length {}", length);
    }
}

/// Padding is libsodium's `sodium_pad` (ISO/IEC 7816-4): `data ‖ 0x80 ‖ 0x00…` up to the block size.
#[test]
fn sodium_pad_format() {
    crate::init().unwrap();

    let iso7816 = |data: &[u8], total: usize| {
        let mut ret = data.to_vec();
        ret.push(0x80);
        ret.resize(total, 0);
        ret
    };

    let padded = buffer_pad_fixed(b"etebase.vcard", 32).unwrap();
    assert_eq!(padded, iso7816(b"etebase.vcard", 32));
    assert_eq!(buffer_unpad_fixed(&padded, 32).unwrap(), b"etebase.vcard");

    // A full block gets a whole extra block of padding.
    let full = [7; 32];
    assert_eq!(buffer_pad_fixed(&full, 32).unwrap(), iso7816(&full, 64));

    assert_eq!(buffer_pad_small(b"abc").unwrap(), iso7816(b"abc", 4));
    assert_eq!(buffer_pad(b"abc").unwrap(), iso7816(b"abc", 1024));
    assert_eq!(buffer_pad(&[]).unwrap(), iso7816(&[], 1024));

    let exact = vec![1; 16384];
    let padded = buffer_pad(&exact).unwrap();
    assert_eq!(padded, iso7816(&exact, 32768));
    assert_eq!(buffer_unpad(&padded).unwrap(), exact);
}

/// End to end: a cached collection recorded with the old build must still decrypt.
///
/// Covers the key hierarchy (account → collection → item), deterministic collection type
/// encryption, metadata MAC/additional data, content chunking with a shuffled index header,
/// chunk padding and the msgpack cache layout.
#[test]
fn recorded_collection() {
    crate::init().unwrap();

    // Non-repeating data: the recording build wrote broken index headers for duplicate chunks.
    let content = randombytes_deterministic(120004, &seq32(5));

    let account = AccountCryptoManager::new(&seq32(4), 1).unwrap();
    let col = EncryptedCollection::cache_load(include_bytes!("golden_vectors/collection.msgpack"))
        .unwrap();
    let cm = col.crypto_manager(&account).unwrap();

    assert!(col.verify(&cm).unwrap());
    assert_eq!(col.collection_type(&account).unwrap(), "etebase.vcard");
    assert_eq!(col.meta(&cm).unwrap(), b"meta bytes");
    assert_eq!(col.item().test_chunk_uids().len(), 8);
    assert_eq!(col.content(&cm).unwrap(), content);
}
