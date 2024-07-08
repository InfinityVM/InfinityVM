package queue

import "errors"

// Sentinel errors that can be returned by the queue.
var (
	ErrQueueEmpty = errors.New("queue is empty")
)

// Queue defines the interface for a queue that can be used to produce types of
// type T.
type Queue[T any] interface {
	// Push adds a job to the queue.
	Push(T) error
	// Pop removes a job from the queue.
	Pop() (T, error)
	// Reset clears the queue.
	Reset() error
	// Listen returns a read-only channel that can be used to listen for new jobs.
	Listen() <-chan T
	// Size returns the number of jobs in the queue.
	Size() int
	// Close closes the queue.
	Close() error
}
