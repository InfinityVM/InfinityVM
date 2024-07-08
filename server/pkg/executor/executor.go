package executor

import (
	"context"
	"encoding/hex"
	"fmt"
	"math"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"

	"github.com/ethos-works/InfinityVM/server/pkg/db"
	"github.com/ethos-works/InfinityVM/server/pkg/queue"
	"github.com/ethos-works/InfinityVM/server/pkg/types"
	"github.com/rs/zerolog"
)

const (
	// DefaultGRPCMaxRecvMsgSize defines the default gRPC max message size in
	// bytes the server can receive.
	DefaultGRPCMaxRecvMsgSize = 1024 * 1024 * 10

	// DefaultGRPCMaxSendMsgSize defines the default gRPC max message size in
	// bytes the server can send.
	DefaultGRPCMaxSendMsgSize = math.MaxInt32

	// QueueSize defines the size of the job queue, which when full, will block.
	QueueSize = 1024
)

// Executor defines the job executor. It is responsible for enqueuing, storing,
// and executing (via the zkShim) jobs. The caller must ensure to start the
// executor, which will consume submitted jobs from the queue.
type Executor struct {
	logger     zerolog.Logger
	db         db.DB
	queue      queue.Queue[*types.Job]
	grpcClient *grpc.ClientConn
}

func New(logger zerolog.Logger, db db.DB, zkClientAddr string) (*Executor, error) {
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
		logger:     logger,
		db:         db,
		queue:      queue.NewMemQueue[*types.Job](QueueSize),
		grpcClient: grpcClient,
	}, nil
}

func (e *Executor) SubmitJob(job *types.Job) error {
	// 1. Store the job in the database.
	// 2. Enqueue the job.
	panic("not implemented!")
}

func (e *Executor) Start(ctx context.Context) error {
	jobCh := e.queue.ListenCh()

	for {
		select {
		case <-ctx.Done():
			_ = e.queue.Close()
			return ctx.Err()

		case job := <-jobCh:
			// 1. Execute the job.
			// 2. Update the job record
			e.logger.Info().Str("program_verifying_key", hex.EncodeToString(job.ProgramVerifyingKey)).Uint32("job_id", job.Id).Msg("executing job...")
		}
	}
}
