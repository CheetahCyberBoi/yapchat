use std::{env, path::PathBuf};

use lazy_static::lazy_static;

// Globals used by the project
// TODO: Migrate this to be a proper Ratatui-style configuration system somehow.
lazy_static! {
    pub static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_uppercase().to_string();
    pub static ref DATA_FOLDER: Option<PathBuf> = 
        env::var(format!("{}_DATA", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);

    pub static ref CONFIG_FOLDER: Option<PathBuf> = 
    env::var(format!("{}_CONFIG", PROJECT_NAME.clone()))
        .ok()
        .map(PathBuf::from);
        
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_name_get() {
        let project_name = PROJECT_NAME.clone();
        assert_eq!(project_name, env!("CARGO_CRATE_NAME").to_uppercase().to_string());
    }

    #[test]
    fn data_dir_get() {
        let _data_dir = DATA_FOLDER.clone().expect("Data folder not found");
    }

    #[test]
    fn config_dir_get() {
        let _config_dir = CONFIG_FOLDER.clone().expect("Config folder not found");
    }
}