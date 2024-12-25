CREATE TABLE IF NOT EXISTS elf_programs (
    program_id BYTEA PRIMARY KEY,
    program_elf BYTEA NOT NULL,
    vm_type INTEGER NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_program_id ON elf_programs(program_id); 