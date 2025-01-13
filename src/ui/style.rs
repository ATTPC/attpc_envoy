use crate::envoy::ecc_operation::ECCStatus;
use crate::envoy::sentry_types::SentryServerStatus;
use eframe::egui::Color32;
use std::cmp::Ordering;

impl From<&ECCStatus> for Color32 {
    fn from(value: &ECCStatus) -> Color32 {
        match value {
            ECCStatus::Offline => Color32::GOLD,
            ECCStatus::Busy => Color32::LIGHT_RED,
            ECCStatus::Idle => Color32::WHITE,
            ECCStatus::Described => Color32::LIGHT_YELLOW,
            ECCStatus::Prepared => Color32::LIGHT_BLUE,
            ECCStatus::Ready => Color32::LIGHT_GREEN,
            ECCStatus::Running => Color32::GREEN,
            _ => Color32::RED,
        }
    }
}

impl From<&SentryServerStatus> for Color32 {
    fn from(value: &SentryServerStatus) -> Color32 {
        match value {
            SentryServerStatus::Offline => Color32::GOLD,
            SentryServerStatus::Online => Color32::GREEN,
            _ => Color32::RED,
        }
    }
}

pub fn n_files_color(n_files: &i32) -> Color32 {
    match n_files.cmp(&0) {
        Ordering::Equal => Color32::LIGHT_GRAY,
        Ordering::Less => Color32::RED,
        Ordering::Greater => Color32::GOLD,
    }
}

pub fn pretty_ellapsed_time(seconds: u64) -> String {
    let hrs = ((seconds as f64) / 3600.0).floor() as u64;
    let mut remainder = seconds - hrs * 3600;
    let mins = ((remainder as f64) / 60.0).floor() as u64;
    remainder -= mins * 60;
    format!("{hrs:02}:{mins:02}:{remainder:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pretty_time() {
        let expected = String::from("01:00:00");
        let value = pretty_ellapsed_time(3600);
        assert_eq!(value, expected);
    }
}
