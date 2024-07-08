package db_test

import (
	"testing"

	"github.com/stretchr/testify/require"

	"github.com/ethos-works/InfinityVM/server/pkg/db"
)

func TestMemDB(t *testing.T) {
	db, err := db.NewMemDB()
	require.NoError(t, err)

	defer func() {
		require.NoError(t, db.Close())
	}()
}
