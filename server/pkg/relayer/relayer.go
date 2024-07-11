package relayer

import (
	"context"

	"github.com/rs/zerolog"

	"github.com/ethos-works/InfinityVM/server/pkg/eth"
	"github.com/ethos-works/InfinityVM/server/pkg/queue"
	"github.com/ethos-works/InfinityVM/server/pkg/types"
)

// Relayer monitors the Infinity coprocessor server for completed jobs and submits them to JobManager contract
type Relayer struct {
	Logger      zerolog.Logger
	Coordinator *Coordinator
}

// Returns a new Relayer
func NewRelayer(logger zerolog.Logger, queueService queue.Queue[*types.Job], ethClient eth.EthClient, workerCount int) *Relayer {
	config := &Config{
		workerCount,
	}
	coordinator := NewCoordinator(config, queueService, ethClient)

	return &Relayer{
		logger,
		coordinator,
	}
}

// Configure and start Relayer
func (r *Relayer) Start(ctx context.Context) error {
	errChan := make(chan error, 1)

	go func() {
		if err := r.Coordinator.Start(); err != nil {
			errChan <- err
		}
	}()

	select {
	case <-ctx.Done():
		r.Logger.Info().Msg("shutting down relayer")
		r.Coordinator.Stop()
		return nil
	case err := <-errChan:
		r.Logger.Error().Err(err).Msg("relayer failure")
		r.Coordinator.Stop()
		return err
	}
}
