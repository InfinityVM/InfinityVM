package queue

import (
	"github.com/ethos-works/InfinityVM/server/pkg/types"
)

// Queue defines the interface for a queue that can be used to produce jobs.
type Queue interface {
	// Push adds a job to the queue.
	Push(j types.Job) error
	// Pop removes a job from the queue.
	Pop() (types.Job, error)
	// Peek returns the next job in the queue without removing it.
	Peek() (types.Job, error)
	// Reset clears the queue.
	Reset() error
	// Size returns the number of jobs in the queue.
	Size() int
}
