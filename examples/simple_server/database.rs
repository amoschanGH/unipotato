use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use crate::models::User;

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Database {
    pub users: Vec<User>,
    pub next_user_id: u32,
}

impl Database {
    pub fn load() -> Self {
        if Path::new("database.json").exists() {
            let content = fs::read_to_string("database.json").unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_else(|_| Self::default_data())
        } else {
            Self::default_data()
        }
    }

    pub fn save(&self) {
        let json = serde_json::to_string_pretty(self).unwrap();
        fs::write("database.json", json).ok();
    }

    fn default_data() -> Self {
        Self {
            users: vec![
                User { id: 1, name: "Alice".to_string(), email: "alice@example.com".to_string() },
                User { id: 2, name: "Bob".to_string(), email: "bob@example.com".to_string() },
                User { id: 3, name: "Charlie".to_string(), email: "charlie@example.com".to_string() },
            ],
            next_user_id: 4,
        }
    }
}
