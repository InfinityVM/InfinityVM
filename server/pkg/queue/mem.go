package queue

// MemQueue is an in-memory thread-safe implementation of the Queue interface
// using FIFO order, using a channel as the underlying data structure. Pushing
// will block if the queue is full.
type MemQueue[T any] struct {
	container chan T
}

func NewMemQueue[T any](size int) Queue[T] {
	return &MemQueue[T]{
		container: make(chan T, size),
	}
}

func (m *MemQueue[T]) Push(x T) error {
	m.container <- x
	return nil
}

func (m *MemQueue[T]) Pop() (T, error) {
	return <-m.container, nil
}

func (m *MemQueue[T]) Reset() error {
	size := cap(m.container)
	close(m.container)

	m.container = make(chan T, size)
	return nil
}

func (m *MemQueue[T]) Size() int {
	return len(m.container)
}

func (m *MemQueue[T]) ListenCh() <-chan T {
	return m.container
}

func (m *MemQueue[T]) Close() error {
	close(m.container)
	return nil
}
