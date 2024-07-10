package executor

import (
	"context"
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"math"
	"sync"

	"github.com/rs/zerolog"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/protobuf/proto"

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

	// DefaultQueueSize defines the size of the job queue, which when full, will block.
	DefaultQueueSize = 1024
)

// Executor defines the job executor. It is responsible for enqueuing, storing,
// and executing (via the zkShim) jobs. The caller must ensure to start the
// executor, which will consume submitted jobs from the queue.
type Executor struct {
	logger         zerolog.Logger
	db             db.DB
	execQueue      queue.Queue[*types.Job]
	broadcastQueue queue.Queue[*types.Job]
	grpcClient     *grpc.ClientConn
}

func New(logger zerolog.Logger, db db.DB, zkClientAddr string, execQueue, broadcastQueue queue.Queue[*types.Job]) (*Executor, error) {
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
		logger:         logger,
		db:             db,
		execQueue:      execQueue,
		broadcastQueue: broadcastQueue,
		grpcClient:     grpcClient,
	}, nil
}

func (e *Executor) SubmitJob(job *types.Job) error {
	job.Status = types.JobStatus_JOB_STATUS_PENDING

	if err := e.SaveJob(job); err != nil {
		return fmt.Errorf("failed to save job: %w", err)
	}

	if err := e.execQueue.Push(job); err != nil {
		return fmt.Errorf("failed to enqueue job: %w", err)
	}

	return nil
}

func (e *Executor) Start(ctx context.Context, numWorkers int) {
	jobCh := e.execQueue.ListenCh()

	wg := new(sync.WaitGroup)
	for i := 0; i < numWorkers; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			e.startWorker(ctx, jobCh)
		}()
	}

	wg.Wait()
}

func (e *Executor) startWorker(ctx context.Context, jobCh <-chan *types.Job) {
	e.logger.Info().Msg("starting worker...")

	for {
		select {
		case <-ctx.Done():
			e.logger.Info().Msg("stopping worker...")
			return

		case job := <-jobCh:
			e.logger.Info().Str("program_verifying_key", hex.EncodeToString(job.ProgramVerifyingKey)).Uint32("job_id", job.Id).Msg("executing job...")

			// TODO(bez): Execute gRPC call to execute job and set fields based on gRPC
			// response.
			// if err := e.grpcClient.Invoke(context.Background(), "/service.ExecuteJob", nil, nil); err != nil {
			// 	e.logger.Error().Err(err).Msg("failed to execute job")

			// 	job.Status = types.JobStatus_JOB_STATUS_FAILED
			// 	if err := e.SaveJob(job); err != nil {
			// 		e.logger.Error().Err(err).Msg("failed to save job")
			// 	}

			// 	continue
			// }

			job.Status = types.JobStatus_JOB_STATUS_DONE
			// job.Result = ...
			// job.ZkvmOperatorAddress = ...
			// job.ZkvmOperatorSignature = ...

			if err := e.SaveJob(job); err != nil {
				e.logger.Error().Err(err).Msg("failed to save job")
				continue
			}

			// push to broadcast queue
			if err := e.broadcastQueue.Push(job); err != nil {
				e.logger.Error().Err(err).Msg("failed to push job to broadcast queue")
			}
		}
	}
}

func (e *Executor) SaveJob(job *types.Job) error {
	idBz := make([]byte, 4)
	binary.BigEndian.PutUint32(idBz, job.Id)

	bz, err := proto.Marshal(job)
	if err != nil {
		return fmt.Errorf("failed to marshal job: %w", err)
	}

	return e.db.Set(idBz, bz)
}

func (e *Executor) GetJob(id uint32) (*types.Job, error) {
	idBz := make([]byte, 4)
	binary.BigEndian.PutUint32(idBz, id)

	bz, err := e.db.Get(idBz)
	if err != nil {
		return nil, fmt.Errorf("failed to get job: %w", err)
	}

	job := new(types.Job)
	if err := proto.Unmarshal(bz, job); err != nil {
		return nil, fmt.Errorf("failed to unmarshal job: %w", err)
	}

	return job, nil
}
