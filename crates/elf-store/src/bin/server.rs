use ivm_elf_store::{elf_store::v1::elf_store_server::ElfStoreServer, ElfStoreService};
use sqlx::{migrate::MigrateDatabase, postgres::PgPoolOptions};
use std::env;
use tonic::transport::Server;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    if !sqlx::Postgres::database_exists(&database_url).await? {
        sqlx::Postgres::create_database(&database_url).await?;
    }

    let pool = PgPoolOptions::new().max_connections(5).connect(&database_url).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let addr = "[::0]:50052".parse()?;
    let service = ElfStoreService::new(pool);

    info!("ElfStore server listening on {}", addr);

    Server::builder().add_service(ElfStoreServer::new(service)).serve(addr).await?;

    Ok(())
}
