use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    // When true, the server will return a 404 error if the query parameter `name` is not found.
    // If false, it will return the first server in the list.
    // This is useful for cases where you want to ensure that a specific server is always used
    // when no `name` is provided, and you want to avoid confusion with multiple servers
    // that might have the same command.
    // Default is false.
    #[serde(default)]
    pub not_found_error: bool,
    // key is language name, value is the command to start the server.
    pub servers: Option<HashMap<String, ServerConfig>>,
    // key is driver name, value is the SQL configuration.
    // Supported drivers are: mysql, postgres, sqlite.
    pub sql: Option<HashMap<String, SqlConfig>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ServerConfig {
    pub command: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SqlConfig {
    pub host: String,
    pub port: u16,
    pub admin_username: String,
    pub admin_password: String,
    pub proto: Option<String>,
}
