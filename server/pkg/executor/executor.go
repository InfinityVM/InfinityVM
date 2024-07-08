package executor

import (
	"context"
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
// and executing (via the zkShim) jobs. The caller must ensure to start the
// executor, which will consume submitted jobs from the queue.
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

func (e *Executor) SubmitJob(job types.Job) error {
	// 1. Store the job in the database.
	// 2. Enqueue the job.
	panic("not implemented!")
}

func (e *Executor) Start(ctx context.Context) error {
	jobCh := e.queue.Listen()

	for {
		select {
		case <-ctx.Done():
			e.queue.Close()
			return ctx.Err()

		case <-jobCh:
			// 1. Dequeue a job.
			// 2. Execute the job.
			// 3. Update the job record
			panic("not implemented!")
		}
	}
}
