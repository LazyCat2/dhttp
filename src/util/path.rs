//! Path utilities

use std::path::{Path, PathBuf};
use std::error::Error;
use std::fmt;
#[cfg(unix)]
use std::ffi::OsStr;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

use percent_encoding_lite::Bitmask;

use crate::core::{HttpError, HttpErrorType};
use crate::reqres::StatusCode;

// ================================================================================

// Why does Windows sanitize_path require valid utf-8?
// Sorry. I'm bad at explaining things, but here is the story:

// To handle non-unicode paths on windows, we have to pass the WTF-8 string into urlencode,
// and decode WTF-8 back into UTF-16 to pass it into NT api
// Unfortunately, std treats such string (OsStr) as opaque api, and function to construct it is unsafe
// I don't want to mess with that
// i.e. read_dir() -> PathBuf -> OsStr -> WTF-8 &[u8] -> urlencode() -> &str
// &str -> urldecode() -> WTF-8 &[u8] -> OsStr -> Path -> File::open
// &[u8] -> OsStr step is flawed, std does not guarantee that internal representation will always be
// WTF-8, BUT it will because &str has to be compatible with OsStr

// Alternatively, we can add fs::read_dir that returns our OsStr, but it is incompatible with PathBuf
// And adding FS apis into a web framework already starts looking weird, even if just for security

// We can go the .encode_wide() roundtrip to utf-16 from wtf-8 from utf-16 but it just looks...
// ...like any other insanely stupid and ugly hack when people try to workaround Rust's incompleteness
// I'd better leave it alone until someone requests supporting non-unicode paths here

// Hopefully I'll fix that flaw if I ever make a no_std async crate

// ================================================================================

// RESOLVE_BENEATH is amazing, but keeping fd instead of opening each time will make a very very
// terrible and confusing thing: moving the host directory out will not stop it from being hosted!!!
// sooo, we have to open it each time
// not a huge deal anyways, just a very important thing to remember
// also since NT doesn't have resolve_beneath it is somewhat useless to call Dir::open on it
// even google safeopen (and Go's stdlib variant) just calls NtCreateFile in loop which does not
// protect from possible directory traversal (oh hell)
// safety measures are good, but a proper sandbox is your best friend
// (maybe in future i'll add LSM dhttp module & crate which allows something similar to unveil(),
// should be more than enough for protection)

// Since for now the only thing that produces PathBuf is fs::read_dir and that consumes it is File::open,
// we can move theirs api into our own crate for that, while being hella overengineered this will
// solve our problem completely

// ON THE OTHER HAND we can ignore its existense and simplify dhttp APIs SOMEHOW
// and that should be hella smart since windows PathBuf conversion is, well, fallible
// We can use OsStr::to_utf8_lossy, while API-wise this is the most convenient solution,
// in reality it will silently hide errors, but oh hell who cares

// Anyways, only Linux support is our no. 1 priority

/// Converts request route into a relative path and checks its safety.
/// Automatically performs url decoding - please keep in mind if you enforce additional checks
///
/// See [`DangerousPathError`] for details of these checks
pub fn sanitize(route: &str) -> Result<PathBuf, DangerousPathError> {
    let decoded = percent_encoding_lite::decode(route);
    #[cfg(windows)]
    return sanitize_win(str::from_utf8(&decoded).map_err(|_| DangerousPathError::InvalidCharacters)?);
    #[cfg(unix)]
    return sanitize_unix(&decoded);
    #[cfg(not(any(windows, unix)))]
    not_implemented
}

#[cfg(unix)]
fn sanitize_unix(route: &[u8]) -> Result<PathBuf, DangerousPathError> {
    if route.contains(&0) { return Err(DangerousPathError::InvalidCharacters); }

    let mut out = PathBuf::new();
    for segment in route.split(|&c| c == b'/') {
        if segment.is_empty() || segment == b"." { continue; }
        if segment == b".." { return Err(DangerousPathError::DangerousPath); }
        out.push(OsStr::from_bytes(segment));
    }

    if out.as_os_str().is_empty() { out.push("."); }
    Ok(out)
}

// TODO: merge two functions once I validate the windows version
#[cfg(windows)]
fn sanitize_win(route: &str) -> Result<PathBuf, DangerousPathError> {
    if route.contains(|c| c < ' ') { return Err(DangerousPathError::InvalidCharacters); }
    // also / is invalid but we are filtering path not filenames
    // but \ is invalid because HTTP routes are separated by /
    if route.contains([':', '\\']) { return Err(DangerousPathError::InvalidCharacters); }

    let mut out = PathBuf::new();
    for segment in route.split('/') {
        if segment.is_empty() || segment == "." { continue; }
        if segment == ".." { return Err(DangerousPathError::DangerousPath); }
        // Findings:
        // nul/123 never exists
        // nul/ is valid as well as nul.txt
        // nultxt is a file
        // any of these listed are existing
        // this should be checked for any lower/upper case!!!
        // + technically we 1) need only last non empty seg, 2) check if it equals one of these AND if it starts with nul. and etc
        // invalid characters (except '\' ':') do not cause any harm BUT ux-wise this error should be bad request instead of very funny localized windows error
        if segment.starts_with("CON") // also CONIN$ CONOUT$
        || segment.starts_with("PRN")
        || segment.starts_with("AUX")
        || segment.starts_with("NUL")
        || segment.starts_with("COM") // 1-9 ¹²³
        || segment.starts_with("LPT") // 1-9 ¹²³
        {
            return Err(DangerousPathError::DangerousPath);
        }
        out.push(segment);
    }

    if out.as_os_str().is_empty() { out.push("."); }
    Ok(out)
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum DangerousPathError {
    /// Path contains dangerous segments (`..` and drive letters on Windows)
    DangerousPath,
    /// Path was either invalid UTF-8 (only on Windows), or contained forbidden characters:
    /// - `\0` on unix
    /// - 0-31 and `<>:"/\|?*` on Windows
    InvalidCharacters,
}

impl fmt::Display for DangerousPathError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DangerousPathError::DangerousPath => f.write_str("path contains `..` or drive letters"),
            DangerousPathError::InvalidCharacters => f.write_str("path contains forbidden characters"),
        }
    }
}

impl Error for DangerousPathError {}
impl HttpError for DangerousPathError {
    fn error_type(&self) -> HttpErrorType { HttpErrorType::Hidden }
    fn status_code(&self) -> StatusCode { StatusCode::BAD_REQUEST }
}

/// Performs URL encoding for a given [`Path`] (lossy on Windows)
pub fn encode(path: &Path) -> String {
    #[cfg(windows)]
    return percent_encoding_lite::encode(&path.to_string_lossy(), Bitmask::PATH);
    #[cfg(unix)]
    return percent_encoding_lite::encode(path.as_os_str().as_bytes(), Bitmask::PATH);
    #[cfg(not(any(windows, unix)))]
    not_implemented
}

// TODO: some unit tests?
