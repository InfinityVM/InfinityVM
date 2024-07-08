package queue_test

import (
	"testing"

	"github.com/ethos-works/InfinityVM/server/pkg/queue"
	"github.com/stretchr/testify/require"
)

func TestMemQueue(t *testing.T) {
	q := queue.NewMemQueue[int]()
	require.Equal(t, 0, q.Size())

	elements := []int{1, 2, 3, 4, 5}
	for _, e := range elements {
		require.NoError(t, q.Push(e))
	}

	for _, e := range elements {
		x, err := q.Peek()
		require.NoError(t, err)
		require.Equal(t, e, x)

		x, err = q.Pop()
		require.NoError(t, err)
		require.Equal(t, e, x)
	}

	require.Equal(t, 0, q.Size())

	_, err := q.Peek()
	require.ErrorIs(t, err, queue.ErrQueueEmpty)
}
