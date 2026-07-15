use std::fs;

use crate::modules::utils::Utils;

pub struct Backuper {
    pub count: usize,
    pub list: Vec<String>,
}

impl Backuper {
    pub fn new() -> Self {
        let list = Self::calculate_list();

        Self {
            count: list.len(),
            list,
        }
    }

    fn calculate_list() -> Vec<String> {
        let mut list = Vec::new();
        let app_dir = Utils::get_backups_dir();
        let app_dir = fs::read_dir(app_dir).expect("Could not read backups directory");

        for entry in app_dir {
            let path = entry
                .expect("Error while reading db directory entry")
                .path();

            if path.is_file() && path.to_str().unwrap().contains("aeternitas_backup_") {
                let file_name = path.file_name().unwrap().to_str().unwrap();

                list.push(String::from(file_name));
            }
        }

        list
    }

    pub fn recalculate_list(&mut self) {
        let new_list = Self::calculate_list();

        self.count = new_list.len();
        self.list = new_list;
    }
}
