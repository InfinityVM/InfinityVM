pub mod elf_store {
    pub mod v1 {
        tonic::include_proto!("elf_store.v1");
    }
}

use elf_store::v1::{
    elf_store_server::ElfStore, GetElfRequest, GetElfResponse, StoreElfRequest, StoreElfResponse,
};
use sqlx::{postgres::PgPool, Row};
use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct ElfStoreService {
    pool: PgPool,
}

impl ElfStoreService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[tonic::async_trait]
impl ElfStore for ElfStoreService {
    async fn store_elf(
        &self,
        request: Request<StoreElfRequest>,
    ) -> Result<Response<StoreElfResponse>, Status> {
        let req = request.into_inner();

        match sqlx::query(
            "INSERT INTO elf_programs (program_id, program_elf, vm_type) VALUES ($1, $2, $3)",
        )
        .bind(&req.program_id)
        .bind(&req.program_elf)
        .bind(req.vm_type)
        .execute(&self.pool)
        .await
        {
            Ok(_) => Ok(Response::new(StoreElfResponse { success: true })),
            Err(e) => Err(Status::internal(format!("Failed to store ELF: {}", e))),
        }
    }

    async fn get_elf(
        &self,
        request: Request<GetElfRequest>,
    ) -> Result<Response<GetElfResponse>, Status> {
        let program_id = request.into_inner().program_id;

        let result =
            sqlx::query("SELECT program_elf, vm_type FROM elf_programs WHERE program_id = $1")
                .bind(program_id.as_slice())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        match result {
            Some(row) => Ok(Response::new(GetElfResponse {
                program_elf: row.get("program_elf"),
                vm_type: row.get("vm_type"),
            })),
            None => Err(Status::not_found("ELF program not found")),
        }
    }
}
