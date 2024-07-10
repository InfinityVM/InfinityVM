package relayer

import (
	"github.com/ethos-works/InfinityVM/server/pkg/eth"
	"github.com/ethos-works/InfinityVM/server/pkg/queue"
	"github.com/ethos-works/InfinityVM/server/pkg/types"
)

// Config holds settings for running the Coordinator
type Config struct {
	// WorkerPoolCount specifies the number of workers in the worker pool used to process broadcasted jobs concurrently
	WorkerPoolCount int
}

// Coordinator schedules and manages the execution of broadcasted jobs from the server, interfacing between the queue
// service and the Ethereum client.
type Coordinator struct {
	Config       *Config
	QueueService queue.Queue[*types.Job]
	EthClient    *eth.EthClient
}

// Returns a new Job Coordinator
func NewCoordinator(c *Config, qs queue.Queue[*types.Job], ec *eth.EthClient) *Coordinator {
	return &Coordinator{
		Config:       c,
		QueueService: qs,
		EthClient:    ec,
	}
}

// Starts the Coordinator Service
func (m *Coordinator) Start() error {
	panic("not implemented!")
}

// Stops the Coordinator Service
func (m *Coordinator) Stop() {
	panic("not implemented!")
}

// Fetch executed Jobs
// nolint:unused
func (m *Coordinator) fetchBroadcastedJobs() {
	panic("not implemented!")
}
