package relayer

import (
	"context"
	"errors"
	"github.com/ethos-works/InfinityVM/server/pkg/queue"
	"github.com/ethos-works/InfinityVM/server/pkg/testutil"
	"github.com/ethos-works/InfinityVM/server/pkg/types"
	"github.com/golang/mock/gomock"
	"github.com/rs/zerolog"
	"github.com/stretchr/testify/require"
	"os"
	"testing"
	"time"
)

func TestRelayerLifecycle(t *testing.T) {
	ctrl := gomock.NewController(t)
	defer ctrl.Finish()

	logger := zerolog.New(os.Stdout)
	broadcastQueue := queue.NewMemQueue[*types.Job](1000)
	ethClient := testutil.NewMockEthClient(ctrl)
	coordinator := testutil.NewMockCoordinator(ctrl)

	relayer := NewRelayer(logger, broadcastQueue, ethClient, 10)
	relayer.Coordinator = coordinator

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	coordinator.EXPECT().Start().Return(nil)
	coordinator.EXPECT().Stop().Return()

	go func() {
		time.Sleep(1 * time.Second)
		cancel()
	}()

	err := relayer.Start(ctx)
	require.NoError(t, err)

	coordinator.EXPECT().Start().Return(errors.New("relayer failure"))
	coordinator.EXPECT().Stop().Return()

	ctx, cancel = context.WithCancel(context.Background())
	defer cancel()

	err = relayer.Start(ctx)
	require.Error(t, err)
}
