//! URL encoding/decoding functions
//!
//! [`encode`] is equivalent to JavaScript's `encodeURIComponent`

// encodeURIComponent:
// A–Z a–z 0–9 - _ . ! ~ * ' ( )
const SAFE_CHARS: &[u8] = &[
    // +32
    0,    b'!', 0,    0,    0,    0,    0,    b'\'', b'(', b')', b'*', 0,    0,    b'-', b'.', 0,
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7',  b'8', b'9', 0,    0,    0,    0,    0,    0,
    0,    b'A', b'B', b'C', b'D', b'E', b'F', b'G',  b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O',
    b'P', b'Q', b'R', b'S', b'T', b'U', b'V', b'W',  b'X', b'Y', b'Z', 0,    0,    0,    0,    b'_',
    0,    b'a', b'b', b'c', b'd', b'e', b'f', b'g',  b'h', b'i', b'j', b'k', b'l', b'm', b'n', b'o',
    b'p', b'q', b'r', b's', b't', b'u', b'v', b'w',  b'x', b'y', b'z', 0,    0,    0,    b'~', 0,
];
// encodeURI:
// adds ; / ? : @ & = + $ , #
// RFC3986:
// adds [ ], removes * ' ( )

// equivalent to encodeURIComponent
fn is_safe(ch: usize) -> bool {
    ch >= 32 && ch - 32 < SAFE_CHARS.len() && SAFE_CHARS[ch - 32] != 0
}

/// Encodes URL component
pub fn encode(src: &[u8]) -> String {
    encode_base(src, false)
}

const HEX: &[u8] = b"0123456789ABCDEF";
pub(crate) fn encode_base(src: &[u8], slash_allowed: bool) -> String {
    let mut out = String::with_capacity(src.len());
    for ch in src.iter().copied() {
        if ch == b' ' {
            out.push('+');
        } else if is_safe(ch as usize) || (slash_allowed && ch == b'/') {
            out.push(ch as char);
        } else {
            out.push('%');
            out.push(HEX[ch as usize >> 4] as char);
            out.push(HEX[ch as usize & 0xF] as char);
        }
    }
    out
}

/// Decodes URL percent-encoding
pub fn decode(src: &str) -> Vec<u8> {
    let mut slice = src.as_bytes();
    let mut out = vec![];
    while let Some(&i) = slice.first() {
        slice = &slice[1..]; // i wish rust had C++ iterators

        if i == b'+' {
            out.push(b' ');
        } else if i != b'%' {
            out.push(i);
        } else {
            if slice.len() < 2 { out.push(i); slice = &slice[1..]; continue; }
            let (hi, lo) = (slice[0], slice[1]);
            let digits = char::from(hi).to_digit(16).zip(char::from(lo).to_digit(16));
            if digits.is_none() { out.push(i); slice = &slice[1..]; continue; }
            let (hi, lo) = digits.unwrap();
            out.push((hi * 16 + lo) as u8);
            slice = &slice[2..];
        }
    }
    out
}

pub use crate::util::path::encode as encode_path;

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn urldecode_test() {
        let encoded = "Anno+1404.Gold+Edition.v+2.1.5010.0.(%D0%9D%D0%BE%D0%B2%D1%8B%D0%B9+%D0%94%D0%B8%D1%81%D0%BA).(2010).Repack";
        let decoded = String::from_utf8(decode(encoded)).unwrap();
        let correct = "Anno 1404.Gold Edition.v 2.1.5010.0.(Новый Диск).(2010).Repack";
        assert_eq!(&decoded, correct);
    }
    #[test]
    fn urlencode_test() {
        let orig = "Microsoft Windows 10, version 22H2, build 19045.2846 (updated April 2023) - Оригинальные образы от Microsoft MSDN [Ru]";
        let encoded = encode(orig.as_bytes());
        let correct = "Microsoft+Windows+10%2C+version+22H2%2C+build+19045.2846+(updated+April+2023)+-+%D0%9E%D1%80%D0%B8%D0%B3%D0%B8%D0%BD%D0%B0%D0%BB%D1%8C%D0%BD%D1%8B%D0%B5+%D0%BE%D0%B1%D1%80%D0%B0%D0%B7%D1%8B+%D0%BE%D1%82+Microsoft+MSDN+%5BRu%5D";
        assert_eq!(&encoded, correct);
    }
}
