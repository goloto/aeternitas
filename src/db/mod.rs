use std::{
    fs,
    path::PathBuf,
};

use rusqlite::{Connection, Error};

use crate::time_formating::TimeFormating;

pub struct Db {
    connection: Connection,
}

pub struct DbProject {
    pub id: i64,
    pub name: String,
}

impl Clone for DbProject {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
        }
    }
}

impl Db {
    pub fn new() -> Self {
        let connection = Connection::open(Db::db_path()).expect("Could not open db");
        let db = Self { connection };
        db.migrate_v1();

        db
    }

    fn db_path() -> PathBuf {
        let mut data_path =
            dirs::data_local_dir().expect("Could not retrieve local data directory");
        data_path.push("aeternitas");
        fs::create_dir_all(&data_path).expect("Could not create directory for db");
        data_path.push("aeternitas");
        data_path.set_extension("db");

        data_path
    }

    fn migrate_v1(&self) {
        match self.connection.execute(
            "CREATE TABLE IF NOT EXISTS projects (
                id   INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE
            );",
            [],
        ) {
            Err(e) => panic!("{e}"),
            _ => {}
        };

        match self.connection.execute(
            "CREATE TABLE IF NOT EXISTS timers (
                id         INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES projects(id),
                started_at INTEGER NOT NULL,
                stopped_at INTEGER DEFAULT NULL
            );",
            [],
        ) {
            Err(e) => panic!("{e}"),
            _ => {}
        }
    }

    pub fn add_new_project(&self, name: &str) {
        self.connection
            .execute("INSERT INTO projects (name) VALUES (?1);", &[name])
            .expect("Could not add new project to db");
    }

    pub fn start_timer(&self, project_id: i64) {
        if self.check_is_running_timer() {
            panic!("There is already running timer!")
        }

        let sys_time = TimeFormating::current_time();
        self.connection
            .execute(
                "INSERT INTO timers (project_id, started_at) VALUES (?1, ?2);",
                [project_id, sys_time],
            )
            .expect("Could not insert new timer to db");
    }

    pub fn stop_timer(&self) {
        if !self.check_is_running_timer() {
            panic!("There is no running timer!")
        }

        let sys_time = TimeFormating::current_time();
        let timer_id: i64 = self
            .connection
            .query_row(
                "SELECT id FROM timers WHERE stopped_at IS NULL;",
                [],
                |row| row.get(0),
            )
            .expect("Could not select id for current timer");
        self.connection
            .execute(
                "UPDATE timers SET stopped_at = ?1 WHERE id = ?2;",
                [sys_time, timer_id],
            )
            .expect("Could not insert new timer to db");
    }

    pub fn projects_list(&self) -> Vec<DbProject> {
        let mut query = self
            .connection
            .prepare("SELECT id, name FROM projects;")
            .expect("Could not select projects from db");
        let projects = query
            .query_map([], |row| {
                Ok(DbProject {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })
            .expect("Error while mapping projects list query");

        let mut projects_vec = Vec::new();

        for item in projects {
            match item {
                Ok(item) => projects_vec.push(item),
                _ => panic!("Error while creating vector with projects"),
            }
        }

        projects_vec
    }

    pub fn check_is_running_timer(&self) -> bool {
        let running_timer_id: i64 = self
            .connection
            .query_row(
                "SELECT id FROM timers WHERE stopped_at IS NULL;",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|_e| -1);

        running_timer_id != -1
    }

    pub fn current_timer(&self) -> i64 {
        let result: Result<i64, Error> = self.connection.query_row(
            "SELECT started_at FROM timers WHERE stopped_at IS NULL;",
            [],
            |row| row.get(0),
        );

        match result {
            Ok(timer) => timer,
            _ => -1,
        }

    }

    pub fn reset(&self) {
        self.connection
            .execute("DROP TABLE IF EXISTS projects", [])
            .expect("Could not drop projects table");
        self.connection
            .execute("DROP TABLE IF EXISTS timers", [])
            .expect("Could not drop timers table");

        self.migrate_v1();
    }
}
