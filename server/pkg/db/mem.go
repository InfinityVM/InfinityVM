package db

import (
	"errors"
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
