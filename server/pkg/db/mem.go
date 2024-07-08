package db

import (
	"fmt"

	"github.com/tidwall/buntdb"
)

type MemDB struct {
	db *buntdb.DB
}

func NewMemDB() (DB, error) {
	db, err := buntdb.Open(":memory:") // in-memory non-durable embedded database
	if err != nil {
		return nil, fmt.Errorf("failed to open db: %w", err)
	}

	return &MemDB{db: db}, nil
}
