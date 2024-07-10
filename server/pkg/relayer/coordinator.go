package relayer

import (
	"sync"

	"github.com/rs/zerolog"
	"github.com/rs/zerolog/log"

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
type Coordinator interface {
	Start() error
	Stop()
}

type InfCoordinator struct {
	Config       *Config
	QueueService queue.Queue[*types.Job]
	EthClient    eth.EthClient
	log          zerolog.Logger
	wg           sync.WaitGroup
	stopChan     chan struct{}
	errChan      chan error
	stopOnce     sync.Once
}

// Returns a new Job Coordinator
func NewCoordinator(c *Config, qs queue.Queue[*types.Job], ec eth.EthClient, log zerolog.Logger) *InfCoordinator {
	return &InfCoordinator{
		Config:       c,
		QueueService: qs,
		EthClient:    ec,
		log:          log,
		stopChan:     make(chan struct{}),
		errChan:      make(chan error, 1),
	}
}

// Starts the Coordinator Service
func (c *InfCoordinator) Start() error {
	for i := 0; i < c.Config.WorkerPoolCount; i++ {
		c.wg.Add(1)
		go c.processBroadcastedJobs()
	}

	go func() {
		c.wg.Wait()
		close(c.errChan)
	}()

	select {
	case <-c.stopChan:
		return nil
	case err := <-c.errChan:
		c.Stop()
		return err
	}
}

// Stops the Coordinator Service
func (c *InfCoordinator) Stop() {
	c.stopOnce.Do(func() {
		close(c.stopChan)
	})
	c.wg.Wait()
	c.log.Info().Msg("stopped relayer coordinator")
}

// Fetch and execute Jobs
func (c *InfCoordinator) processBroadcastedJobs() {
	defer c.wg.Done()
	for {
		select {
		case <-c.stopChan:
			return
		default:
			if c.QueueService.Size() >= 1 {
				job, err := c.QueueService.Pop()
				if err != nil {
					c.log.Error().Msgf("error fetching latest job from broadcast queue: %v", err)
					continue
				}
				err = c.EthClient.ExecuteCallback(job)
				if err != nil {
					c.log.Error().Msgf("error executing eth callback: %v", err)
					if _, ok := err.(*eth.FatalClientError); ok {
						c.errChan <- err
						return
					}
					continue
				}
				log.Info().Msg("successfully executed eth callback")
			}
		}
	}
}
