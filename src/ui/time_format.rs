pub fn pretty_ellapsed_time(seconds: u64) -> String {
    let hrs = ((seconds as f64) / 3600.0).floor() as u64;
    let mut remainder = seconds - hrs * 3600;
    let mins = ((seconds as f64) / 60.0).floor() as u64;
    remainder -= mins * 60;
    format!("{hrs:02}:{mins:02}:{remainder:02}")
}
