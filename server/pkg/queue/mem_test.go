package queue_test

import (
	"testing"

	"github.com/stretchr/testify/require"

	"github.com/ethos-works/InfinityVM/server/pkg/queue"
)

func TestMemQueue(t *testing.T) {
	q := queue.NewMemQueue[int](100)
	require.Equal(t, 0, q.Size())

	elements := []int{1, 2, 3, 4, 5}
	for _, e := range elements {
		require.NoError(t, q.Push(e))
	}

	for _, e := range elements {
		x, err := q.Pop()
		require.NoError(t, err)
		require.Equal(t, e, x)
	}

	require.Equal(t, 0, q.Size())

	for _, e := range elements {
		require.NoError(t, q.Push(e))
	}

	err := q.Reset()
	require.NoError(t, err)
}
