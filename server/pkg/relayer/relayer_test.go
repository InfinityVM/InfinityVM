package relayer

import (
	"context"
	"encoding/binary"
	"fmt"
	"os"
	"testing"
	"time"

	"github.com/rs/zerolog"
	"github.com/stretchr/testify/require"
	"go.uber.org/mock/gomock"
	"google.golang.org/protobuf/proto"

	"github.com/ethos-works/InfinityVM/server/pkg/db"
	"github.com/ethos-works/InfinityVM/server/pkg/eth"
	"github.com/ethos-works/InfinityVM/server/pkg/executor"
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

	db, err := db.NewMemDB()
	require.NoError(t, err)

	relayer := NewRelayer(logger, broadcastQueue, ethClient, db, 10, executor.SaveJob)
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	for i := 1; i <= 3; i++ {
		job := &types.Job{
			Id: uint32(i),
		}
		err := broadcastQueue.Push(job)
		require.NoError(t, err)
		txId := make([]byte, job.Id)
		ethClient.EXPECT().ExecuteCallback(ctx, job).Return(txId, nil).Times(1)
	}
	require.Equal(t, broadcastQueue.Size(), 3)

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
			for i := 1; i <= 3; i++ {
				j, err := getJob(db, uint32(i))
				require.NoError(t, err)
				require.Equal(t, j.TransactionHash, make([]byte, i))
			}
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

	db, err := db.NewMemDB()
	require.NoError(t, err)

	job := &types.Job{}
	err = broadcastQueue.Push(job)
	require.NoError(t, err)

	relayer := NewRelayer(logger, broadcastQueue, ethClient, db, 10, executor.SaveJob)
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	ethClient.EXPECT().ExecuteCallback(ctx, job).Return(nil, eth.NewFatalClientError("service unavailable")).Times(1)

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

func getJob(db db.DB, id uint32) (*types.Job, error) {
	idBz := make([]byte, 4)
	binary.BigEndian.PutUint32(idBz, id)

	bz, err := db.Get(idBz)
	if err != nil {
		return nil, fmt.Errorf("failed to get job: %w", err)
	}

	job := new(types.Job)
	if err := proto.Unmarshal(bz, job); err != nil {
		return nil, fmt.Errorf("failed to unmarshal job: %w", err)
	}

	return job, nil
}
