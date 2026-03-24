/// Decompose a Unix timestamp (seconds since epoch) into
/// `(year, month 1-12, day 1-31, hour, minute, second)`.
pub(crate) fn decompose_unix_secs(secs: u64) -> (u32, u32, u32, u32, u32, u32) {
    let ss = (secs % 60) as u32;
    let mm = ((secs / 60) % 60) as u32;
    let hh = ((secs / 3_600) % 24) as u32;
    let mut days = secs / 86_400;

    let mut year = 1970u32;
    loop {
        let dy: u64 = if is_leap_year(year) { 366 } else { 365 };
        if days < dy {
            break;
        }
        days -= dy;
        year += 1;
    }

    let month_days: [u64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u32;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }

    (year, month, (days + 1) as u32, hh, mm, ss)
}

fn is_leap_year(y: u32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
