package relayer

import (
	"context"
	"sync"

	"github.com/rs/zerolog"
	"github.com/rs/zerolog/log"

	"github.com/ethos-works/InfinityVM/server/pkg/eth"
	"github.com/ethos-works/InfinityVM/server/pkg/queue"
	"github.com/ethos-works/InfinityVM/server/pkg/types"
)

// Relayer monitors the Infinity coprocessor server for completed jobs and submits them to JobManager contract
type Relayer struct {
	EthClient       eth.EthClient
	Logger          zerolog.Logger
	workerPoolCount int
	broadcastQueue  queue.Queue[*types.Job]
	wg              sync.WaitGroup
	stopChan        chan struct{}
	errChan         chan error
	stopOnce        sync.Once
}

// Returns a new Relayer
func NewRelayer(logger zerolog.Logger, queueService queue.Queue[*types.Job], ethClient eth.EthClient, workerCount int) *Relayer {
	return &Relayer{
		EthClient:       ethClient,
		Logger:          logger,
		workerPoolCount: workerCount,
		broadcastQueue:  queueService,
		stopChan:        make(chan struct{}),
		errChan:         make(chan error, 1),
	}
}

// Configure and start Relayer
func (r *Relayer) Start(ctx context.Context) error {
	for i := 0; i < r.workerPoolCount; i++ {
		r.wg.Add(1)
		go r.processBroadcastedJobs()
	}

	select {
	case <-ctx.Done():
		r.Logger.Info().Msg("shutting down relayer service")
		r.Stop()
		return nil
	case err := <-r.errChan:
		r.Logger.Error().Err(err).Msg("relayer service failure")
		r.Stop()
		return err
	}
}

func (r *Relayer) Stop() {
	r.stopOnce.Do(func() {
		close(r.stopChan)
	})
	r.wg.Wait()
	r.Logger.Info().Msg("stopped relayer coordinator")
}

// Fetch and execute Jobs
func (r *Relayer) processBroadcastedJobs() {
	defer r.wg.Done()
	for {
		select {
		case <-r.stopChan:
			return
		default:
			if r.broadcastQueue.Size() >= 1 {
				job, err := r.broadcastQueue.Pop()
				if err != nil {
					r.Logger.Error().Msgf("error fetching latest job from broadcast queue: %v", err)
					continue
				}
				err = r.EthClient.ExecuteCallback(job)
				if err != nil {
					r.Logger.Error().Msgf("error executing eth callback: %v", err)
					if _, ok := err.(*eth.FatalClientError); ok {
						r.errChan <- err
						return
					}
					continue
				}
				log.Info().Msg("successfully executed eth callback")
			}
		}
	}
}
