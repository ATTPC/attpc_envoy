use super::error::ConfigError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, Read, Write};
use std::path::PathBuf;

const DEFAULT_FIELDS: [&str; 11] = [
    "Target Gas",
    "Beam",
    "Energy (MeV/U)",
    "Pressure (Torr)",
    "B-Field (T)",
    "V_THGEM (V)",
    "V_MM (V)",
    "V_Cathode (kV)",
    "E-Drift (V)",
    "E-Trans (V)",
    "GET Freq. (MHz)",
];

/// (De)Serializable application configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip)]
    pub path: PathBuf,

    pub experiment: String,
    pub run_number: i32,
    pub description: String,
    pub fields: BTreeMap<String, String>,
}

impl Config {
    pub fn new() -> Self {
        let mut fields = BTreeMap::new();
        for field in DEFAULT_FIELDS {
            fields.insert(field.to_string(), String::default());
        }
        Config {
            path: PathBuf::from("example.yml"),
            experiment: String::from("Exp"),
            run_number: 0,
            description: String::from("Write here"),
            fields,
        }
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let mut file = File::create(&self.path)?;
        let yaml_str = serde_yaml::to_string::<Config>(self)?;
        file.write_all(yaml_str.as_bytes())?;
        Ok(())
    }

    pub fn load(&mut self, path: PathBuf) -> Result<(), ConfigError> {
        let mut file = File::open(&path)?;
        let mut yaml_str = String::new();
        file.read_to_string(&mut yaml_str)?;
        *self = serde_yaml::from_str::<Config>(&yaml_str)?;
        self.path = path;
        Ok(())
    }

    pub fn add_field(&mut self, field: String, value: String) {
        self.fields.insert(field, value);
    }

    /// Get the path to a configuration table which we will log experiment data to
    fn get_config_table(&self) -> PathBuf {
        let mut header = String::from("Run,Note,Duration");
        for key in self.fields.keys() {
            header = format!("{header},{key}");
        }
        header = format!("{header}\n");
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
                match file.write_all(header.as_bytes()) {
                    Ok(_) => (),
                    Err(e) => {
                        tracing::error!("Could not write header to config table: {}", e);
                    }
                }
            }
        } else {
            let mut lines = Vec::new();
            if let Ok(file) = std::fs::File::open(&table_path) {
                let mut reader = std::io::BufReader::new(file);
                let mut header_line = String::new();
                reader.read_line(&mut header_line).expect("No header line!");
                if format!("{header_line}\n") == header {
                    return table_path;
                }
                lines = match reader.lines().collect() {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::error!("Could not read existing table! Error: {}", e);
                        return table_path;
                    }
                }
            }
            if let Ok(mut file) = std::fs::File::create(&table_path) {
                match file.write_all(header.as_bytes()) {
                    Ok(_) => (),
                    Err(e) => {
                        tracing::error!(
                            "Could not write header to run log file at reformat: {}",
                            e
                        );
                        return table_path;
                    }
                }
                for line in lines {
                    let line_end = format!("{line}\n");
                    match file.write_all(line_end.as_bytes()) {
                        Ok(_) => (),
                        Err(e) => {
                            tracing::error!(
                                "Could not write line {} when reformating run log: {}",
                                line,
                                e
                            );
                            return table_path;
                        }
                    }
                }
            }
        }

        table_path
    }

    /// Write experiment data to a log table
    pub fn write_table(&self, ellapsed_time: std::time::Duration) {
        let path = self.get_config_table();
        let mut row = format!(
            "{},{},{}",
            self.run_number,
            self.description,
            ellapsed_time.as_secs()
        );
        if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(path) {
            for field in self.fields.values() {
                row = format!("{row},{field}")
            }
            row = format!("{row}\n");
            match file.write_all(row.as_bytes()) {
                Ok(_) => (),
                Err(e) => {
                    tracing::error!("Could not write row to config table: {}", e);
                }
            }
        }
    }
}
