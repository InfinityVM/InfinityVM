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

const (
	// DefaultWorkerCount is the number of concurrent workers available to process broadcasted Jobs
	DefaultWorkerCount = 3
)

// Relayer monitors the Infinity coprocessor server for completed jobs and submits them to JobManager contract
type Relayer struct {
	EthClient       eth.EthClientI
	Logger          zerolog.Logger
	workerPoolCount int
	broadcastQueue  queue.Queue[*types.Job]
	wg              sync.WaitGroup
	errChan         chan error
}

// Returns a new Relayer
func NewRelayer(logger zerolog.Logger, queueService queue.Queue[*types.Job], ethClient eth.EthClientI, workerCount int) *Relayer {
	return &Relayer{
		EthClient:       ethClient,
		Logger:          logger,
		workerPoolCount: workerCount,
		broadcastQueue:  queueService,
		errChan:         make(chan error, 1),
	}
}

// Configure and start Relayer
func (r *Relayer) Start(ctx context.Context) error {
	r.Logger.Info().Msg("starting relayer...")

	for i := 0; i < r.workerPoolCount; i++ {
		r.wg.Add(1)
		go r.processBroadcastedJobs(ctx)
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
	r.wg.Wait()
	r.Logger.Info().Msg("stopping relayer...")
}

// Fetch and execute Jobs
func (r *Relayer) processBroadcastedJobs(ctx context.Context) {
	defer r.wg.Done()
	r.Logger.Info().Msg("starting relayer worker...")

	for {
		select {
		case <-ctx.Done():
			return
		default:
			if r.broadcastQueue.Size() >= 1 {
				job, err := r.broadcastQueue.Pop()
				if err != nil {
					r.Logger.Error().Msgf("error fetching latest job from broadcast queue: %v", err)
					continue
				}
				err = r.EthClient.ExecuteCallback(ctx, job)
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
