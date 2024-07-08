package queue

// Queue defines the interface for a queue that can be used to produce types of
// type T.
type Queue[T any] interface {
	// Push adds a job to the queue.
	Push(j T) error
	// Pop removes a job from the queue.
	Pop() (T, error)
	// Peek returns the next job in the queue without removing it.
	Peek() (T, error)
	// Reset clears the queue.
	Reset() error
	// Size returns the number of jobs in the queue.
	Size() int
}
