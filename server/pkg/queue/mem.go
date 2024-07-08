package queue

import (
	"container/list"
	"sync"
)

// MemQueue is an in-memory thread-safe implementation of the Queue interface
// using FIFO order.
type MemQueue[T any] struct {
	mu        sync.RWMutex
	container *list.List
}

func NewMemQueue[T any]() Queue[T] {
	return &MemQueue[T]{
		container: list.New(),
	}
}

func (m *MemQueue[T]) Push(x T) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	m.container.PushBack(x)
	return nil
}

func (m *MemQueue[T]) Pop() (T, error) {
	m.mu.Lock()
	defer m.mu.Unlock()

	var result T

	x := m.container.Front()
	if x == nil {
		return result, ErrQueueEmpty
	}

	result = x.Value.(T)

	_ = m.container.Remove(x)
	return result, nil
}

func (m *MemQueue[T]) Peek() (T, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	var result T

	x := m.container.Front()
	if x == nil {
		return result, ErrQueueEmpty
	}

	result = x.Value.(T)
	return result, nil
}

func (m *MemQueue[T]) Reset() error {
	m.mu.Lock()
	defer m.mu.Unlock()

	m.container = m.container.Init()
	return nil
}

func (m *MemQueue[T]) Size() int {
	m.mu.RLock()
	defer m.mu.RUnlock()

	return m.container.Len()
}
