package db

import (
	"errors"
	"fmt"

	"github.com/tidwall/buntdb"
)

// MemDB defines an in-memory non-durable key/value store using an embedded B-tree
// based database.
//
// TODO(bez): Determine if we need to define any custom indexes. Since we're only
// storing jobs for now, this is not necessary.
//
// Ref: https://github.com/tidwall/buntdb?tab=readme-ov-file#custom-indexes
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

func (db *MemDB) Set(key, val []byte) error {
	err := db.db.Update(func(tx *buntdb.Tx) error {
		_, _, err := tx.Set(string(key), string(val), nil)
		return err
	})
	if err != nil {
		return fmt.Errorf("failed to set key: %w", err)
	}

	return nil
}

func (db *MemDB) Get(key []byte) ([]byte, error) {
	var result string

	err := db.db.View(func(tx *buntdb.Tx) error {
		val, err := tx.Get(string(key))
		if err != nil {
			return err
		}

		result = val
		return nil
	})
	if err != nil {
		if errors.Is(err, buntdb.ErrNotFound) {
			return nil, nil
		}

		return nil, fmt.Errorf("failed to get key: %w", err)
	}

	return []byte(result), nil
}

func (db *MemDB) Has(key []byte) (bool, error) {
	err := db.db.View(func(tx *buntdb.Tx) error {
		_, err := tx.Get(string(key))
		return err
	})
	if err != nil {
		if errors.Is(err, buntdb.ErrNotFound) {
			return false, nil
		}

		return false, fmt.Errorf("failed to get key: %w", err)
	}

	return true, nil
}

func (db *MemDB) Delete(key []byte) error {
	err := db.db.Update(func(tx *buntdb.Tx) error {
		_, err := tx.Delete(string(key))
		return err
	})
	if err != nil {
		if errors.Is(err, buntdb.ErrNotFound) {
			return nil
		}

		return fmt.Errorf("failed to delete key: %w", err)
	}

	return nil
}

func (db *MemDB) Close() error {
	return db.db.Close()
}
