use crate::{
    config::Config,
    lsp::{Message, Request},
};

pub struct SqlsDatabase {
    id: String,
    driver: String,
    admin_username: String,
    admin_password: String,
    host: String,
    port: u16,
    created_database: Option<String>,
    created_user: Option<String>,
    created_password: Option<String>,
}

impl SqlsDatabase {
    fn new(
        driver: String,
        admin_username: String,
        admin_password: String,
        host: String,
        port: u16,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        SqlsDatabase {
            id,
            driver,
            admin_username,
            admin_password,
            host,
            port,
            created_database: None,
            created_user: None,
            created_password: None,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    async fn init(
        &mut self,
        init_sql: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self.driver.as_str() {
            "mysql" => self.init_mysql(init_sql).await,
            "postgres" => self.init_postgres(init_sql).await,
            "sqlite" => self.init_sqlite(init_sql).await,
            _ => Err(format!("Unsupported driver: {}", self.driver).into()),
        }
    }

    async fn init_mysql(
        &mut self,
        init_sql: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use sqlx::MySqlPool;

        let db_name = format!("lsp_db_{}", &self.id[..8]);
        let user_name = format!("lsp_user_{}", &self.id[..8]);
        let password = format!("lsp_pass_{}", &self.id[..8]);

        let admin_url = format!(
            "mysql://{}:{}@{}:{}/mysql",
            self.admin_username, self.admin_password, self.host, self.port
        );
        let admin_pool = MySqlPool::connect(&admin_url).await?;
        let mut tx = admin_pool.begin().await?;
        match self
            .create_mysql_resources(&mut tx, &db_name, &user_name, &password)
            .await
        {
            Ok(_) => {
                // 提交管理员事务
                tx.commit().await?;

                // 记录创建的资源
                self.created_database = Some(db_name.clone());
                self.created_user = Some(user_name.clone());
                self.created_password = Some(password.clone());

                // 连接到新创建的数据库执行初始化SQL
                let user_url = format!(
                    "mysql://{}:{}@{}:{}/{}",
                    user_name, password, self.host, self.port, db_name
                );

                let user_pool = MySqlPool::connect(&user_url).await?;
                let mut user_tx = user_pool.begin().await?;

                match sqlx::query(init_sql).execute(&mut *user_tx).await {
                    Ok(_) => {
                        user_tx.commit().await?;
                        println!("MySQL database and init SQL executed successfully");
                        Ok(())
                    }
                    Err(e) => {
                        user_tx.rollback().await?;
                        // 清理创建的资源
                        self.cleanup_mysql_resources().await?;
                        Err(format!("Init SQL execution failed: {}", e).into())
                    }
                }
            }
            Err(e) => {
                tx.rollback().await?;
                Err(e)
            }
        }
    }

    async fn create_mysql_resources(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
        db_name: &str,
        user_name: &str,
        password: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 创建数据库
        let create_db_sql = format!("CREATE DATABASE IF NOT EXISTS `{}`", db_name);
        sqlx::query(&create_db_sql).execute(&mut **tx).await?;

        // 创建用户
        let create_user_sql = format!(
            "CREATE USER IF NOT EXISTS '{}'@'%' IDENTIFIED BY '{}'",
            user_name, password
        );
        sqlx::query(&create_user_sql).execute(&mut **tx).await?;

        // 授权
        let grant_sql = format!(
            "GRANT ALL PRIVILEGES ON `{}`.* TO '{}'@'%'",
            db_name, user_name
        );
        sqlx::query(&grant_sql).execute(&mut **tx).await?;

        // 刷新权限
        sqlx::query("FLUSH PRIVILEGES").execute(&mut **tx).await?;

        Ok(())
    }

    async fn cleanup_mysql_resources(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let admin_url = format!(
            "mysql://{}:{}@{}:{}/mysql",
            self.admin_username, self.admin_password, self.host, self.port
        );

        let admin_pool = sqlx::MySqlPool::connect(&admin_url).await?;

        if let Some(db_name) = &self.created_database {
            let drop_db_sql = format!("DROP DATABASE IF EXISTS `{}`", db_name);
            sqlx::query(&drop_db_sql).execute(&admin_pool).await?;
            self.created_database = None;
        }

        if let Some(user_name) = &self.created_user {
            let drop_user_sql = format!("DROP USER IF EXISTS '{}'@'%'", user_name);
            sqlx::query(&drop_user_sql).execute(&admin_pool).await?;
            self.created_user = None;
        }

        Ok(())
    }

    async fn init_postgres(
        &mut self,
        init_sql: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use sqlx::PgPool;

        let db_name = format!("lsp_db_{}", &self.id[..8]);
        let user_name = format!("lsp_user_{}", &self.id[..8]);
        let password = format!("lsp_pass_{}", &self.id[..8]);

        // 连接到PostgreSQL服务器
        let admin_url = format!(
            "postgres://{}:{}@{}:{}/postgres",
            self.admin_username, self.admin_password, self.host, self.port
        );

        let admin_pool = PgPool::connect(&admin_url).await?;

        // PostgreSQL不支持DDL事务，需要手动管理回滚
        match self
            .create_postgres_resources(&admin_pool, &db_name, &user_name, &password)
            .await
        {
            Ok(_) => {
                self.created_database = Some(db_name.clone());
                self.created_user = Some(user_name.clone());
                self.created_password = Some(password.clone());

                // 连接到新数据库执行初始化SQL
                let user_url = format!(
                    "postgres://{}:{}@{}:{}/{}",
                    user_name, password, self.host, self.port, db_name
                );

                let user_pool = PgPool::connect(&user_url).await?;
                let mut tx = user_pool.begin().await?;

                match sqlx::query(init_sql).execute(&mut *tx).await {
                    Ok(_) => {
                        tx.commit().await?;
                        println!("PostgreSQL database and init SQL executed successfully");
                        Ok(())
                    }
                    Err(e) => {
                        tx.rollback().await?;
                        // 清理创建的资源
                        self.cleanup_postgres_resources().await?;
                        Err(format!("Init SQL execution failed: {}", e).into())
                    }
                }
            }
            Err(e) => Err(e),
        }
    }

    async fn create_postgres_resources(
        &self,
        pool: &sqlx::PgPool,
        db_name: &str,
        user_name: &str,
        password: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 创建用户
        let create_user_sql = format!("CREATE USER {} WITH PASSWORD '{}'", user_name, password);
        sqlx::query(&create_user_sql).execute(pool).await?;

        // 创建数据库
        let create_db_sql = format!("CREATE DATABASE {} OWNER {}", db_name, user_name);
        sqlx::query(&create_db_sql).execute(pool).await?;

        Ok(())
    }

    async fn cleanup_postgres_resources(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let admin_url = format!(
            "postgres://{}:{}@{}:{}/postgres",
            self.admin_username, self.admin_password, self.host, self.port
        );

        let admin_pool = sqlx::PgPool::connect(&admin_url).await?;

        if let Some(db_name) = &self.created_database {
            // 断开数据库连接
            let terminate_sql = format!(
                "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}'",
                db_name
            );
            let _ = sqlx::query(&terminate_sql).execute(&admin_pool).await;

            let drop_db_sql = format!("DROP DATABASE IF EXISTS {}", db_name);
            sqlx::query(&drop_db_sql).execute(&admin_pool).await?;
            self.created_database = None;
        }

        if let Some(user_name) = &self.created_user {
            let drop_user_sql = format!("DROP USER IF EXISTS {}", user_name);
            sqlx::query(&drop_user_sql).execute(&admin_pool).await?;
            self.created_user = None;
        }

        Ok(())
    }

    async fn init_sqlite(
        &mut self,
        init_sql: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use sqlx::SqlitePool;

        let db_name = format!("lsp_db_{}.db", &self.id[..8]);
        let db_path = format!("/tmp/{}", db_name);

        // SQLite 不需要创建用户，直接创建数据库文件
        let url = format!("sqlite://{}", db_path);

        let pool = SqlitePool::connect(&url).await?;
        let mut tx = pool.begin().await?;

        match sqlx::query(init_sql).execute(&mut *tx).await {
            Ok(_) => {
                tx.commit().await?;
                self.created_database = Some(db_path);
                println!("SQLite database created and init SQL executed successfully");
                Ok(())
            }
            Err(e) => {
                tx.rollback().await?;
                // 删除数据库文件
                if let Err(_) = std::fs::remove_file(&db_path) {
                    eprintln!("Failed to cleanup SQLite database file: {}", db_path);
                }
                Err(format!("SQLite init SQL execution failed: {}", e).into())
            }
        }
    }

    pub async fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self.driver.as_str() {
            "mysql" => self.cleanup_mysql_resources().await,
            "postgres" => self.cleanup_postgres_resources().await,
            "sqlite" => {
                if let Some(db_path) = &self.created_database {
                    if let Err(e) = std::fs::remove_file(db_path) {
                        eprintln!("Failed to remove SQLite database file: {}", e);
                    }
                    self.created_database = None;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

/// Extract the database name from the message
/// ```json
/// {
///   "initializationOptions": {
///     "connectionConfig": {
///       "alias": "alias",
///       "driver": "mysql/sqlite/postgres",
///       "dataSourceName": "root:root@tcp(127.0.0.1:13306)/world",
///     },
///     "init": {
///       "driver": "mysql/sqlite/postgres",
///       "initSql": "CREATE TABLE IF NOT EXISTS test (id INTEGER PRIMARY KEY, name TEXT);",
///     }
/// }
/// ```
pub async fn create_database_on_init(
    msg: &mut Message,
    name: &str,
    config: Option<&Config>,
) -> Result<Option<SqlsDatabase>, Box<dyn std::error::Error + Send + Sync>> {
    if name != "sql" {
        return Ok(None);
    }

    match msg {
        Message::Request(request) => match request {
            Request::Initialize { id: _, params: p } => {
                if let Some(v) = &p.initialization_options {
                    let init_sql_str = v
                        .get("init")
                        .and_then(|init| init.get("initSql"))
                        .and_then(|sql| sql.as_str())
                        .unwrap_or("");
                    let driver = v
                        .get("init")
                        .and_then(|config| config.get("driver"))
                        .and_then(|d| d.as_str())
                        .unwrap_or("");
                    let config = config
                        .and_then(|c| c.sql.as_ref())
                        .and_then(|sql| sql.get(driver));

                    if init_sql_str == "" || driver == "" {
                        return Ok(None);
                    }

                    // create the database
                    if let Some(sql_config) = config {
                        let mut db = SqlsDatabase::new(
                            driver.to_string(),
                            sql_config.admin_username.clone(),
                            sql_config.admin_password.clone(),
                            sql_config.host.clone(),
                            sql_config.port,
                        );
                        db.init(init_sql_str).await?;

                        // reset connectionConfig
                        let mut connection_config = serde_json::Map::new();
                        connection_config.insert("driver".into(), driver.into());
                        if let Some(pwd) = &db.created_password {
                            connection_config.insert("passwd".into(), pwd.as_str().into());
                        }
                        if let Some(user) = &db.created_user {
                            connection_config.insert("user".into(), user.as_str().into());
                        }
                        if let Some(created_database) = &db.created_database {
                            connection_config
                                .insert("dbName".into(), created_database.as_str().into());
                        }
                        if let Some(proto) = &sql_config.proto {
                            connection_config.insert("proto".into(), proto.as_str().into());
                        }
                        connection_config.insert("host".into(), sql_config.host.as_str().into());
                        connection_config.insert("port".into(), sql_config.port.into());
                        p.initialization_options = Some(serde_json::json!({
                            "connectionConfig": connection_config,
                        }));
                        return Ok(Some(db));
                    }
                };
            }
            _ => {}
        },
        _ => {}
    };

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp::{types::Id, Message, Request};
    use lsp_types::{ClientCapabilities, InitializeParams};
    use serde_json::json;

    #[tokio::test]
    async fn test_mysql_init() {
        let init_options = json!({
            "connectionConfig": {
                "alias": "test_mysql",
                "driver": "mysql",
                "dataSourceName": "root:password@tcp(127.0.0.1:3306)/mysql"
            },
            "init": {
                "driver": "mysql",
                "initSql": "CREATE TABLE IF NOT EXISTS users (id INT AUTO_INCREMENT PRIMARY KEY, name VARCHAR(100), email VARCHAR(100));"
            }
        });

        let mut message = Message::Request(Request::Initialize {
            id: Id::Number(1),
            params: InitializeParams {
                process_id: None,
                root_uri: None,
                initialization_options: Some(init_options),
                capabilities: ClientCapabilities {
                    text_document: None,
                    workspace: None,
                    window: None,
                    experimental: None,
                    general: None,
                },
                trace: None,
                workspace_folders: None,
                client_info: None,
                locale: None,
                #[allow(warnings)]
                root_path: None,
            },
        });

        let config = Config {
            not_found_error: false,
            servers: None,
            sql: Some(
                [(
                    "mysql".to_string(),
                    crate::config::SqlConfig {
                        host: "127.0.0.1".to_string(),
                        port: 3306,
                        admin_username: "root".to_string(),
                        admin_password: "root".to_string(),
                        proto: Some("tcp".to_string()),
                    },
                )]
                .into(),
            ),
        };

        let data_base = create_database_on_init(&mut message, "sql", Some(&config))
            .await
            .unwrap();
        assert!(data_base.is_some());
        let mut db = data_base.unwrap();
        assert_eq!(db.driver, "mysql");
        assert!(db.created_database.is_some());
        assert!(db.created_user.is_some());
        assert!(db.created_password.is_some());
        assert!(db.created_database.as_ref().unwrap().contains("lsp_db_"));

        match message {
            Message::Request(Request::Initialize { params, .. }) => {
                if let Some(options) = params.initialization_options {
                    if let Some(connection_config) = options.get("connectionConfig") {
                        assert_eq!(connection_config.get("driver").unwrap(), "mysql");
                        assert!(connection_config
                            .get("passwd")
                            .is_some_and(|p| p.as_str().unwrap().contains("lsp_pass_")));
                        assert!(connection_config
                            .get("user")
                            .is_some_and(|u| u.as_str().unwrap().contains("lsp_user_")));
                        assert!(connection_config
                            .get("dbName")
                            .is_some_and(|d| d.as_str().unwrap().contains("lsp_db_")));
                        if let Some(proto) = connection_config.get("proto") {
                            assert_eq!(proto.as_str().unwrap(), "tcp");
                        }
                    }
                }
            }
            _ => panic!("Expected Initialize request"),
        }

        // Clean up the database
        db.cleanup().await.unwrap();
    }
}
