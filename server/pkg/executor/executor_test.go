package executor_test

import (
	"context"
	"testing"
	"time"

	"github.com/stretchr/testify/require"
	"go.uber.org/mock/gomock"

	"github.com/ethos-works/InfinityVM/server/pkg/db"
	"github.com/ethos-works/InfinityVM/server/pkg/executor"
	"github.com/ethos-works/InfinityVM/server/pkg/mock"
	"github.com/ethos-works/InfinityVM/server/pkg/queue"
	"github.com/ethos-works/InfinityVM/server/pkg/testutils"
	"github.com/ethos-works/InfinityVM/server/pkg/types"
)

func TestExecutor(t *testing.T) {
	db, err := db.NewMemDB()
	require.NoError(t, err)

	logger := testutils.NewTestLogger(t)
	execQueue := queue.NewMemQueue[*types.Job](1024)
	broadcastQueue := queue.NewMemQueue[*types.Job](1024)

	mockCtrl := gomock.NewController(t)
	defer mockCtrl.Finish()

	executorClientMock := mock.NewMockZkvmExecutorClient(mockCtrl)
	exec := executor.New(logger, db, executorClientMock, execQueue, broadcastQueue)

	// create cancelable context and start executor
	ctx, cancel := context.WithCancel(context.Background())
	doneCh := make(chan struct{})
	go func() {
		defer close(doneCh)
		exec.Start(ctx, 4)
	}()

	job := &types.Job{
		Id:                  1,
		ProgramVerifyingKey: []byte{0xFF, 0xFF},
		Input:               []byte{0x01, 0x02},
	}

	err = exec.SubmitJob(job)
	require.NoError(t, err)

	executorClientMock.EXPECT().Execute(gomock.Any(), gomock.Any(), gomock.Any()).Return(&types.ExecuteResponse{}, nil)

	require.Eventually(t, func() bool {
		job, err := exec.GetJob(job.Id)
		require.NoError(t, err)

		return job.Status == types.JobStatus_JOB_STATUS_DONE
	}, time.Second, 100*time.Millisecond)

	// ensure graceful cleanup
	cancel()
	<-doneCh
}
