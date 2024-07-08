package db_test

import (
	"fmt"
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

	err = db.Delete([]byte("key001"))
	require.NoError(t, err)

	for i := 0; i < 100; i++ {
		key := []byte(fmt.Sprintf("key%03d", i))
		val := []byte(fmt.Sprintf("val%03d", i))

		err = db.Set(key, val)
		require.NoError(t, err)

		result, err := db.Get(key)
		require.NoError(t, err)
		require.Equal(t, val, result)
	}
}
