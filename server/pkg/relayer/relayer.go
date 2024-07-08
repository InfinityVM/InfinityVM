package relayer

import (
	"context"

	"github.com/rs/zerolog"

	"github.com/ethos-works/InfinityVM/server/pkg/eth"
)

// Configure and start Relayer
func Start(ctx context.Context, logger zerolog.Logger, queueService interface{}, ethClient *eth.EthClient, workerCount int) error {
	// Create Manager Config
	config := &Config{
		logger,
		workerCount,
	}

	// Configure Manager
	manager := NewManager(config, queueService, ethClient)

	errChan := make(chan error, 1)

	// Start Manager
	go func() {
		if err := manager.Start(); err != nil {
			errChan <- err
		}
	}()

	select {
	case <-ctx.Done():
		logger.Info().Msg("shutting down relayer")
		manager.Stop()
		return nil
	case err := <-errChan:
		logger.Error().Err(err).Msg("relayer failure")
		manager.Stop()
		return err
	}
}
