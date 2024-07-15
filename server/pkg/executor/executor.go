package executor

import (
	"context"
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"sync"

	"github.com/rs/zerolog"
	"google.golang.org/protobuf/proto"

	"github.com/ethos-works/InfinityVM/server/pkg/db"
	"github.com/ethos-works/InfinityVM/server/pkg/queue"
	"github.com/ethos-works/InfinityVM/server/pkg/types"
)

const (
	// DefaultQueueSize defines the size of the job queue, which when full, will block.
	DefaultQueueSize = 1024

	// DefaultWorkerCount defines the number of workers to start for processing jobs.
	DefaultWorkerCount = 4
)

// Executor defines the job executor. It is responsible for enqueuing, storing,
// and executing (via the zkShim) jobs. The caller must ensure to start the
// executor, which will consume submitted jobs from the queue.
type Executor struct {
	logger         zerolog.Logger
	db             db.DB
	execQueue      queue.Queue[*types.Job]
	broadcastQueue queue.Queue[*types.Job]
	zkClient       types.ZkvmExecutorClient
	wg             sync.WaitGroup
}

func New(logger zerolog.Logger, db db.DB, zkClient types.ZkvmExecutorClient, execQueue, broadcastQueue queue.Queue[*types.Job]) *Executor {
	return &Executor{
		logger:         logger,
		db:             db,
		execQueue:      execQueue,
		broadcastQueue: broadcastQueue,
		zkClient:       zkClient,
		wg:             sync.WaitGroup{},
	}
}

func (e *Executor) SubmitJob(job *types.Job) error {
	ok, err := e.HasJob(job.Id)
	if err != nil {
		return fmt.Errorf("failed to check if job exists: %w", err)
	}
	if ok {
		return fmt.Errorf("job with ID %d already exists", job.Id)
	}

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

	for i := 0; i < numWorkers; i++ {
		e.wg.Add(1)
		go e.startWorker(ctx, jobCh)
	}

	e.wg.Wait()
}

func (e *Executor) startWorker(ctx context.Context, jobCh <-chan *types.Job) {
	defer e.wg.Done()
	e.logger.Info().Msg("starting worker...")

	for {
		select {
		case <-ctx.Done():
			e.logger.Info().Msg("stopping worker...")
			return

		case job := <-jobCh:
			logger := e.logger.With().Str("program_verifying_key", hex.EncodeToString(job.ProgramVerifyingKey)).Uint32("job_id", job.Id).Logger()
			logger.Info().Msg("executing job...")

			req := &types.ExecuteRequest{
				Inputs: &types.JobInputs{
					JobId:               job.Id,
					MaxCycles:           job.MaxCycles,
					VmType:              job.VmType,
					ProgramVerifyingKey: job.ProgramVerifyingKey,
					ProgramInput:        job.Input,
				},
			}

			resp, err := e.zkClient.Execute(context.Background(), req)
			if err != nil {
				logger.Error().Err(err).Msg("failed to execute job")

				job.Status = types.JobStatus_JOB_STATUS_FAILED
				if err := e.SaveJob(job); err != nil {
					logger.Error().Err(err).Msg("failed to save job")
				}

				continue
			}

			logger.Info().Str("result", hex.EncodeToString(resp.ResultWithMetadata)).Msg("job executed successfully")

			job.Status = types.JobStatus_JOB_STATUS_DONE
			job.Result = resp.ResultWithMetadata
			job.ZkvmOperatorAddress = resp.ZkvmOperatorAddress
			job.ZkvmOperatorSignature = resp.ZkvmOperatorSignature

			if err := e.SaveJob(job); err != nil {
				logger.Error().Err(err).Msg("failed to save job")
				continue
			}

			// push to broadcast queue
			logger.Info().Msg("pushing finished job to broadcast queue")
			if err := e.broadcastQueue.Push(job); err != nil {
				logger.Error().Err(err).Msg("failed to push job to broadcast queue")
			}
		}
	}
}

// SubmitELF submits an ELF to the ZK shim and returns a verification key.
func (e *Executor) SubmitELF(elf []byte, vmType types.VmType) ([]byte, error) {
	req := &types.CreateElfRequest{
		ProgramElf: elf,
		VmType:     vmType,
	}

	resp, err := e.zkClient.CreateElf(context.Background(), req)
	if err != nil {
		return nil, fmt.Errorf("failed to submit ELF program: %w", err)
	}

	return resp.VerifyingKey, nil
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

func (e *Executor) HasJob(id uint32) (bool, error) {
	idBz := make([]byte, 4)
	binary.BigEndian.PutUint32(idBz, id)

	return e.db.Has(idBz)
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
