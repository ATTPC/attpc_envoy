use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;

const HEADER_STR: &str = "Run,Duration(s),Note,Gas,Beam,Energy(MeV/U),Pressure(Torr),B-Field(T),V_THGEM(V),V_MM(V),V_Cathode(kV),E-Drift(V),E-Trans(V)\n";

/// (De)Serializable application configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip)]
    pub config_path: PathBuf,

    pub experiment: String,
    pub run_number: i32,
    pub description: String,
    pub pressure: f32,
    pub v_thgem: f32,
    pub v_mm: f32,
    pub e_drift: f32,
    pub v_cathode: f32,
    pub e_trans: f32,
    pub gas: String,
    pub beam: String,
    pub energy: f32,
    pub magnetic_field: f32,
}

impl Config {
    pub fn new() -> Self {
        Config {
            config_path: PathBuf::from("example.yml"),
            experiment: String::from("Exp"),
            run_number: 0,
            description: String::from("Write here"),
            pressure: 0.0,
            v_thgem: 0.0,
            v_mm: 0.0,
            e_drift: 0.0,
            v_cathode: 0.0,
            e_trans: 0.0,
            gas: String::from("H2"),
            beam: String::from("16C"),
            energy: 0.0,
            magnetic_field: 0.0,
        }
    }

    /// Get the path to a configuration table which we will log experiment data to
    fn get_config_table(&self) -> PathBuf {
        let table_dir = PathBuf::from("tables/");
        if !table_dir.exists() {
            match std::fs::create_dir(&table_dir) {
                Ok(()) => (),
                Err(e) => tracing::error!(
                    "Could not create table directory due to: {}. The config table will not be saved!",
                    e
                ),
            }
        }

        let table_path = table_dir.join(format!("{}.csv", self.experiment));
        if !table_path.exists() {
            if let Ok(mut file) = std::fs::File::create(&table_path) {
                match file.write_all(HEADER_STR.as_bytes()) {
                    Ok(_) => (),
                    Err(e) => {
                        tracing::error!("Could not write header to config table: {}", e);
                    }
                }
            }
        }

        table_path
    }

    /// Write experiment data to a log table
    pub fn write_table(&self, ellapsed_time: std::time::Duration) {
        let path = self.get_config_table();
        if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(path) {
            let row = format!(
                "{},{},{},{},{},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2}\n",
                self.run_number,
                ellapsed_time.as_secs(),
                self.description,
                self.gas,
                self.beam,
                self.energy,
                self.pressure,
                self.magnetic_field,
                self.v_thgem,
                self.v_mm,
                self.v_cathode,
                self.e_drift,
                self.e_trans
            );
            match file.write_all(row.as_bytes()) {
                Ok(_) => (),
                Err(e) => {
                    tracing::error!("Could not write row to config table: {}", e);
                }
            }
        }
    }
}
