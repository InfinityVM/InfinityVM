package relayer

import (
	"context"
	"os"
	"testing"
	"time"

	"github.com/rs/zerolog"
	"github.com/stretchr/testify/require"
	"go.uber.org/mock/gomock"

	"github.com/ethos-works/InfinityVM/server/pkg/eth"
	"github.com/ethos-works/InfinityVM/server/pkg/mock"
	"github.com/ethos-works/InfinityVM/server/pkg/queue"
	"github.com/ethos-works/InfinityVM/server/pkg/types"
)

func TestRelayerLifecycle(t *testing.T) {
	ctrl := gomock.NewController(t)
	defer ctrl.Finish()

	logger := zerolog.New(os.Stdout)
	broadcastQueue := queue.NewMemQueue[*types.Job](1000)
	ethClient := mock.NewMockEthClientI(ctrl)
	errChan := make(chan error, 1)

	relayer := NewRelayer(logger, broadcastQueue, ethClient, 10)

	for i := 1; i <= 3; i++ {
		job := &types.Job{}
		err := broadcastQueue.Push(job)
		require.NoError(t, err)
		ethClient.EXPECT().ExecuteCallback(job).Return(nil).Times(1)

	}
	require.Equal(t, broadcastQueue.Size(), 3)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go func() {
		errChan <- relayer.Start(ctx)
	}()

	time.Sleep(1 * time.Second)
	cancel()

	require.Eventually(t, func() bool {
		select {
		case err := <-errChan:
			require.NoError(t, err)
			require.Equal(t, broadcastQueue.Size(), 0)
			return true
		default:
			return false
		}
	}, 3*time.Second, 100*time.Millisecond)
}

func TestProcessBroadcastedJobFailure(t *testing.T) {
	ctrl := gomock.NewController(t)
	defer ctrl.Finish()

	logger := zerolog.New(os.Stdout)
	broadcastQueue := queue.NewMemQueue[*types.Job](100)
	ethClient := mock.NewMockEthClientI(ctrl)
	errChan := make(chan error, 1)

	job := &types.Job{}
	err := broadcastQueue.Push(job)
	require.NoError(t, err)

	relayer := NewRelayer(logger, broadcastQueue, ethClient, 10)
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	ethClient.EXPECT().ExecuteCallback(job).Return(eth.NewFatalClientError("service unavailable")).Times(1)

	go func() {
		errChan <- relayer.Start(ctx)
	}()

	time.Sleep(1 * time.Second)
	cancel()

	require.Eventually(t, func() bool {
		select {
		case err := <-errChan:
			require.Error(t, err)
			return true
		default:
			return false
		}
	}, 2*time.Second, 100*time.Millisecond)
}
