package executor

import (
	"fmt"
	"math"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"

	"github.com/ethos-works/InfinityVM/server/pkg/db"
	"github.com/ethos-works/InfinityVM/server/pkg/queue"
	"github.com/ethos-works/InfinityVM/server/pkg/types"
)

const (
	// DefaultGRPCMaxRecvMsgSize defines the default gRPC max message size in
	// bytes the server can receive.
	DefaultGRPCMaxRecvMsgSize = 1024 * 1024 * 10

	// DefaultGRPCMaxSendMsgSize defines the default gRPC max message size in
	// bytes the server can send.
	DefaultGRPCMaxSendMsgSize = math.MaxInt32
)

// Executor defines the job executor. It is responsible for enqueuing, storing,
// and executing (via the zkShim) jobs.
type Executor struct {
	db         db.DB
	queue      queue.Queue[types.Job]
	grpcClient *grpc.ClientConn
}

func New(db db.DB, zkClientAddr string) (*Executor, error) {
	grpcClient, err := grpc.NewClient(
		zkClientAddr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithDefaultCallOptions(
			grpc.MaxCallRecvMsgSize(DefaultGRPCMaxRecvMsgSize),
			grpc.MaxCallSendMsgSize(DefaultGRPCMaxSendMsgSize),
		),
	)
	if err != nil {
		return nil, fmt.Errorf("failed to create gRPC client: %w", err)
	}

	return &Executor{
		db:         db,
		queue:      queue.NewMemQueue[types.Job](),
		grpcClient: grpcClient,
	}, nil
}
