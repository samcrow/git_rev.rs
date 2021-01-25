extern crate proc_macro;
use proc_macro::TokenStream;

use std::convert::AsRef;
use std::process::Command;

/// Number of bytes used to store a complete git commit hash
const HASH_BYTES: usize = 20;

/// A commit hash
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
// Internally, bytes are ordered from most to least significant (big endian).
struct Hash([u8; HASH_BYTES]);

impl Hash {
    /// Returns the 64 most significant bits of this hash as a u64
    pub fn most_significant_u64(&self) -> u64 {
        // This is a lot of code, but it generally compiles down to a few loads
        // and byte swaps.
        u64::from(self.0[0]) << 56
            | u64::from(self.0[1]) << 48
            | u64::from(self.0[2]) << 40
            | u64::from(self.0[3]) << 32
            | u64::from(self.0[4]) << 24
            | u64::from(self.0[5]) << 16
            | u64::from(self.0[6]) << 8
            | u64::from(self.0[7])
    }
}

impl From<Hash> for [u8; HASH_BYTES] {
    /// Converts a hash into an array of bytes, with the most significant byte first
    fn from(hash: Hash) -> Self {
        hash.0
    }
}

impl AsRef<[u8; HASH_BYTES]> for Hash {
    /// Converts a hash reference to a reference to an array of bytes, with the most significant
    /// byte first
    fn as_ref(&self) -> &[u8; HASH_BYTES] {
        &self.0
    }
}

impl std::fmt::Display for Hash {
    /// Displays a hash as 20 hexadecimal characters, the same format used by git
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0.iter() {
            write!(f, "{:02x}", *byte)?;
        }
        Ok(())
    }
}

/// Expands to an Option<u64> containing the first 16 digits of the git HEAD revision code,
/// or None if the revision could not be determined
///
/// # Examples
///
/// ```
/// const REVISION: Option<u64> = git_rev::try_revision_u64!();
/// ```
///
#[proc_macro]
pub fn try_revision_u64(_item: TokenStream) -> TokenStream {
    match get_git_revision() {
        Ok(revision) => {
            let code = format!("Some::<u64>({})", revision.most_significant_u64());
            code.parse().unwrap()
        }
        Err(_e) => "None::<u64>".parse().unwrap(),
    }
}

/// Expands to the first 16 digits of the git HEAD revision code as a u64,
/// or panics (causing a compile failure) if the commit hash could not be determined
///
/// # Examples
///
/// ```ignore
/// const REVISION: u64 = git_rev::revision_u64!();
/// ```
///
#[proc_macro]
pub fn revision_u64(_item: TokenStream) -> TokenStream {
    let revision = get_git_revision().expect("couldn't get git revision");
    format!("{}u64", revision.most_significant_u64())
        .parse()
        .unwrap()
}

/// Expands to an `Option<&'static str>` containing the current HEAD commit hash as a string,
/// or None if the commit hash could not be determined
///
/// # Examples
///
/// ```
/// const REVISION: Option<&'static str> = git_rev::try_revision_string!();
/// ```
///
#[proc_macro]
pub fn try_revision_string(_item: TokenStream) -> TokenStream {
    match get_git_revision() {
        Ok(hash) => format!("Some::<&'static str>(\"{}\")", hash)
            .parse()
            .unwrap(),
        Err(_) => "None::<&'static str>".parse().unwrap(),
    }
}

/// Expands to a string literal containing the current HEAD commit hash as a string,
/// or panics (causing a compile failure) if the commit hash could not be determined
///
/// # Examples
///
/// ```ignore
/// const REVISION: &'static str = git_rev::revision_string!();
/// ```
///
#[proc_macro]
pub fn revision_string(_item: TokenStream) -> TokenStream {
    let revision = get_git_revision().expect("couldn't get git revision");
    format!("\"{}\"", revision).parse().unwrap()
}

fn get_git_revision() -> Result<Hash, Error> {
    // Run git rev-parse HEAD to get the revision
    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .map_err(|_| Error::Command)?;
    if output.status.success() {
        let mut stdout = output.stdout;
        // Expected stdout length is HASH_BYTES * 2 characters, plus one for the newline
        if stdout.len() < HASH_BYTES * 2 {
            return Err(Error::Parse);
        }
        stdout.truncate(HASH_BYTES * 2);
        let stdout = String::from_utf8(stdout).map_err(|_| Error::Parse)?;
        // Parse one byte at a time
        let mut hash_bytes = [0u8; HASH_BYTES];
        for i in 0..HASH_BYTES {
            let two_digits: &str = &stdout[i * 2..i * 2 + 2];
            hash_bytes[i] = u8::from_str_radix(two_digits, 16).map_err(|_| Error::Parse)?;
        }
        Ok(Hash(hash_bytes))
    } else {
        Err(Error::StatusCode)
    }
}

#[derive(Debug)]
enum Error {
    /// Failed to execute git
    Command,
    /// git returned a non-success exit code
    StatusCode,
    /// The output from git could not be parsed
    Parse,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Command => write!(f, "Failed to execute git"),
            Error::StatusCode => write!(f, "git returned a non-success exit code"),
            Error::Parse => write!(f, "the output of git could not be parsed"),
        }
    }
}
