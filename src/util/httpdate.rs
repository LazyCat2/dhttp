//! Utilities to format an HTTP date
//! # Example
//! ```
//! # use dhttp::util::httpdate;
//! # use dhttp::reqres::HttpResponse;
//! # let mut res = HttpResponse::new();
//! let your_time;
//! # your_time = std::fs::metadata("src/util/httpdate.rs").unwrap().modified().unwrap();
//! if let Some(date) = httpdate::from_systime(your_time) {
//!     res.add_header("Last-Modified", date);
//! }
//! ```

use std::time::{SystemTime, UNIX_EPOCH};

use chrono_lite::{time_t, Tm, gmtime, time};

const WEEKDAYS: &[&str] = &["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: &[&str] = &["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

fn httpdate(tm: Tm) -> String {
    let Tm { tm_wday, tm_mday, tm_mon, tm_year, tm_hour, tm_min, tm_sec, .. } = tm;
    let weekday = WEEKDAYS[tm_wday as usize];
    let month = MONTHS[tm_mon as usize];
    let year = tm_year + 1900;
    // example output: Tue, 25 Feb 2025 21:05:51 GMT
    format!("{weekday}, {tm_mday} {month} {year} {tm_hour:02}:{tm_min:02}:{tm_sec:02} GMT")
}

/// Formats an HTTP date from a [`SystemTime`]
///
/// Returns `None` when date formatting fails (i. e. when provided timestamp was invalid)
pub fn from_systime(systime: SystemTime) -> Option<String> {
    let time = systime.duration_since(UNIX_EPOCH).ok()?.as_secs();
    let tm = gmtime(time as time_t)?;

    Some(httpdate(tm))
}

/// Returns the current time in an HTTP date
///
/// May return `None` on Windows after 31 Dec, 3000
pub fn now() -> Option<String> {
    let tm = gmtime(time())?;
    Some(httpdate(tm))
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_httpdate() {
        assert_eq!("Wed, 26 Feb 2025 22:10:59 GMT", &httpdate(gmtime(1740607859).unwrap()));
    }
}
