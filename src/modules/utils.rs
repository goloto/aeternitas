use std::path::PathBuf;

pub struct Utils {}

impl Utils {
    pub fn get_app_dir() -> PathBuf {
        let mut data_path =
            dirs::data_local_dir().expect("Could not retrieve local data directory");
        data_path.push("aeternitas");

        data_path
    }

    pub fn get_backups_dir() -> PathBuf {
        let mut data_path = Utils::get_app_dir();
        data_path.push("backups");

        data_path
    }
}
