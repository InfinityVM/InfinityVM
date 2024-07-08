package queue

import (
	"container/list"
)

type MemQueue[T any] struct {
	container *list.List
}

func NewMemQueue[T any]() Queue[T] {
	return &MemQueue[T]{
		container: list.New(),
	}
}

func (m *MemQueue[T]) Push(x T) error {
	m.container.PushFront(x)
	return nil
}

func (m *MemQueue[T]) Pop() (T, error) {
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
	var result T

	x := m.container.Front()
	if x == nil {
		return result, ErrQueueEmpty
	}

	result = x.Value.(T)
	return result, nil
}

func (m *MemQueue[T]) Reset() error {
	m.container = m.container.Init()
	return nil
}

func (m *MemQueue[T]) Size() int {
	return m.container.Len()
}
