use crate::envoy::ecc_operation::ECCStatus;
use crate::envoy::surveyor_status::{SurveyorDiskStatus, SurveyorStatus};
use eframe::egui::Color32;

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

impl From<&SurveyorStatus> for Color32 {
    fn from(value: &SurveyorStatus) -> Color32 {
        match value {
            SurveyorStatus::Offline => Color32::GOLD,
            SurveyorStatus::Online => Color32::GREEN,
            _ => Color32::RED,
        }
    }
}

impl From<&SurveyorDiskStatus> for Color32 {
    fn from(value: &SurveyorDiskStatus) -> Color32 {
        match value {
            SurveyorDiskStatus::Filled => Color32::GOLD,
            SurveyorDiskStatus::Empty => Color32::GREEN,
            SurveyorDiskStatus::NA => Color32::LIGHT_GRAY,
        }
    }
}

pub fn pretty_ellapsed_time(seconds: u64) -> String {
    let hrs = ((seconds as f64) / 3600.0).floor() as u64;
    let mut remainder = seconds - hrs * 3600;
    let mins = ((seconds as f64) / 60.0).floor() as u64;
    remainder -= mins * 60;
    format!("{hrs:02}:{mins:02}:{remainder:02}")
}
