package relayer

import (
	"context"
	"github.com/ethos-works/InfinityVM/server/pkg/eth"
	"github.com/ethos-works/InfinityVM/server/pkg/queue"
	"github.com/ethos-works/InfinityVM/server/pkg/testutil"
	"github.com/golang/mock/gomock"
	"github.com/rs/zerolog"
	"github.com/stretchr/testify/require"
	"os"
	"testing"
	"time"
)

func TestCoordinatorLifecycle(t *testing.T) {
	ctrl := gomock.NewController(t)
	defer ctrl.Finish()

	logger := zerolog.New(os.Stdout)
	broadcastQueue := queue.NewMemQueue[interface{}](100)
	ethClient := testutil.NewMockEthClient(ctrl)

	config := &Config{3}
	coordinator := NewCoordinator(config, broadcastQueue, ethClient, logger)

	go func() {
		err := coordinator.Start()
		require.NoError(t, err)
	}()

	time.Sleep(1 * time.Second)
	coordinator.Stop()
}

func TestProcessBroadcastedJobsSuccess(t *testing.T) {
	ctrl := gomock.NewController(t)
	defer ctrl.Finish()

	logger := zerolog.New(os.Stdout)
	broadcastQueue := queue.NewMemQueue[interface{}](100)
	ethClient := testutil.NewMockEthClient(ctrl)

	job := "job1"
	err := broadcastQueue.Push(job)
	require.NoError(t, err)

	config := &Config{3}
	coordinator := NewCoordinator(config, broadcastQueue, ethClient, logger)

	ethClient.EXPECT().ExecuteCallback(job).Return(nil).Times(1)

	go func() {
		err := coordinator.Start()
		require.NoError(t, err)
	}()

	time.Sleep(2 * time.Second)
	coordinator.Stop()
}

func TestProcessBroadcastedJobFailure(t *testing.T) {
	ctrl := gomock.NewController(t)
	defer ctrl.Finish()

	logger := zerolog.New(os.Stdout)
	broadcastQueue := queue.NewMemQueue[interface{}](100)
	ethClient := testutil.NewMockEthClient(ctrl)

	job := "job1"
	err := broadcastQueue.Push(job)
	require.NoError(t, err)

	config := &Config{3}
	coordinator := NewCoordinator(config, broadcastQueue, ethClient, logger)

	ethClient.EXPECT().ExecuteCallback(job).Return(eth.NewFatalClientError("service unavailable")).Times(1)

	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	done := make(chan struct{})

	go func() {
		defer close(done)
		err = coordinator.Start()
		require.Error(t, err)
	}()

	select {
	case <-done:
	case <-ctx.Done():
		t.Fatal("timed out")
	}

	coordinator.Stop()
}
