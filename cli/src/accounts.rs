//! Offline account storage: one active account name, plus its offline UUID.

use crate::paths;
use basalt_engine::manifest::Platform;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub player_name: String,
    pub uuid: String,
}

/// Mojang's offline UUID convention: UUID v3 of "OfflinePlayer:<name>".
pub fn offline_uuid(name: &str) -> String {
    let md5 = md5_of(format!("OfflinePlayer:{name}").as_bytes());
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        md5[0], md5[1], md5[2], md5[3], md5[4], md5[5],
        (md5[6] & 0x0f) | 0x30, (md5[7] & 0x3f) | 0x80,
        md5[8], md5[9], md5[10], md5[11], md5[12], md5[13], md5[14], md5[15]
    )
}

/// MD5 is required by the offline UUID convention; small and local only.
fn md5_of(data: &[u8]) -> [u8; 16] {
    // Minimal MD5 implementation (RFC 1321) to avoid a crypto dependency for
    // a non-security hash.
    let mut h: [u32; 4] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476];
    let s: [usize; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6,
        10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    let k: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];

    let mut message = data.to_vec();
    let original_len = (data.len() as u64) * 8;
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&original_len.to_le_bytes());

    for chunk in message.chunks(64) {
        let mut m = [0u32; 16];
        for (i, block) in chunk.chunks(4).enumerate() {
            m[i] = u32::from_le_bytes([block[0], block[1], block[2], block[3]]);
        }
        let (mut a, mut b, mut c, mut d) = (h[0], h[1], h[2], h[3]);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f2 = f
                .wrapping_add(a)
                .wrapping_add(k[i])
                .wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f2.rotate_left(s[i] as u32));
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
    }
    let mut out = [0u8; 16];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    out
}

/// Reads the stored account; None when no account exists yet.
pub fn get() -> Option<Account> {
    let raw = fs::read_to_string(paths::accounts_file()).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Stores the account under the given offline name.
pub fn set(name: &str) -> i32 {
    let account = Account {
        player_name: name.to_string(),
        uuid: offline_uuid(name),
    };
    paths::ensure_dir(&paths::data_dir());
    let raw = serde_json::to_string_pretty(&account).expect("account serializes");
    fs::write(paths::accounts_file(), raw).expect("write accounts.json");
    println!("account stored: {} ({})", account.player_name, account.uuid);
    0
}

/// The account for a launch: stored one, or a fresh one for the given name.
pub fn for_name(name: Option<&str>) -> Option<Account> {
    match name {
        Some(n) => Some(Account {
            player_name: n.to_string(),
            uuid: offline_uuid(n),
        }),
        None => get(),
    }
}

/// The engine platform this build targets.
pub fn platform() -> Platform {
    if cfg!(target_arch = "aarch64") {
        Platform::LinuxArm64
    } else {
        Platform::LinuxX8664
    }
}
