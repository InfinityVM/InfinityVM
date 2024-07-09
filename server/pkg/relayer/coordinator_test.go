package relayer

import (
	"context"
	"os"
	"testing"
	"time"

)

func TestCoordinatorLifecycle(t *testing.T) {
	ctrl := gomock.NewController(t)
	defer ctrl.Finish()

	logger := zerolog.New(os.Stdout)
	broadcastQueue := queue.NewMemQueue[interface{}]()
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
	broadcastQueue := queue.NewMemQueue[interface{}]()
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
	broadcastQueue := queue.NewMemQueue[interface{}]()
	ethClient := testutil.NewMockEthClient(ctrl)

	job := "job1"
	err := broadcastQueue.Push(job)
	require.NoError(t, err)

	config := &Config{3}
	coordinator := NewCoordinator(config, broadcastQueue, ethClient, logger)

	ethClient.EXPECT().ExecuteCallback(job).Return(eth.NewFatalClientError("service unavailable")).Times(1)

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	done := make(chan struct{})

	go func() {
		err = coordinator.Start()
		require.Error(t, err)
		close(done)
	}()

	select {
	case <-done:
	case <-ctx.Done():
		t.Fatal("timed out")
	}

	coordinator.Stop()
}
