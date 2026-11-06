const KIB: u64 = 1024;
const MIB: u64 = KIB * 1024;

pub fn format_size(bytes: u64) -> String {
    if bytes < KIB {
        format!("{bytes} B")
    } else if bytes < MIB {
        format!("{:.1} KB", bytes as f64 / KIB as f64)
    } else {
        format!("{:.1} MB", bytes as f64 / MIB as f64)
    }
}

pub fn format_duration(secs: f64) -> String {
    let total = secs.max(0.0).round() as u64;
    let (h, m, s) = (total / 3600, total % 3600 / 60, total % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_under_one_kib_is_shown_in_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn size_from_one_kib_is_shown_in_kb_with_one_decimal() {
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1024 * 1024 - 1), "1024.0 KB");
    }

    #[test]
    fn size_from_one_mib_is_shown_in_mb_without_larger_units() {
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(5 * 1024 * 1024 + 512 * 1024), "5.5 MB");
        assert_eq!(format_size(3 * 1024 * 1024 * 1024), "3072.0 MB");
    }

    #[test]
    fn duration_under_one_hour_is_minutes_and_padded_seconds() {
        assert_eq!(format_duration(0.0), "0:00");
        assert_eq!(format_duration(9.0), "0:09");
        assert_eq!(format_duration(65.4), "1:05");
        assert_eq!(format_duration(3599.0), "59:59");
    }

    #[test]
    fn duration_from_one_hour_includes_padded_minutes() {
        assert_eq!(format_duration(3600.0), "1:00:00");
        assert_eq!(format_duration(3725.0), "1:02:05");
    }

    #[test]
    fn duration_rounds_to_the_nearest_second_and_clamps_negatives() {
        assert_eq!(format_duration(59.6), "1:00");
        assert_eq!(format_duration(-3.0), "0:00");
    }
}
