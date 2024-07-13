package relayer

import (
	"context"
	"encoding/binary"
	"fmt"
	"sync"


	"github.com/rs/zerolog"
	"google.golang.org/protobuf/proto"

	"github.com/ethos-works/InfinityVM/server/pkg/db"
	"github.com/ethos-works/InfinityVM/server/pkg/eth"
	"github.com/ethos-works/InfinityVM/server/pkg/queue"
	"github.com/ethos-works/InfinityVM/server/pkg/types"
)

const (
	// DefaultWorkerCount is the number of concurrent workers available to process broadcasted Jobs
	DefaultWorkerCount = 3
)

// Relayer monitors the Infinity coprocessor server for completed jobs and submits them to JobManager contract
type Relayer struct {
	EthClient       eth.EthClientI
	Logger          zerolog.Logger
	db              db.DB
	workerPoolCount int
	broadcastQueue  queue.Queue[*types.Job]
	wg              sync.WaitGroup
	errChan         chan error
}

// Returns a new Relayer
func NewRelayer(logger zerolog.Logger, queueService queue.Queue[*types.Job], ethClient eth.EthClientI, db db.DB, workerCount int) *Relayer {
	return &Relayer{
		EthClient:       ethClient,
		Logger:          logger,
		db:              db,
		workerPoolCount: workerCount,
		broadcastQueue:  queueService,
		errChan:         make(chan error, 1),
	}
}

// Configure and start Relayer
func (r *Relayer) Start(ctx context.Context) error {
	r.Logger.Info().Msg("starting relayer...")

	for i := 0; i < r.workerPoolCount; i++ {
		r.wg.Add(1)
		go r.processBroadcastedJobs(ctx)
	}

	select {
	case <-ctx.Done():
		r.Logger.Info().Msg("shutting down relayer service")
		r.Stop()
		return nil
	case err := <-r.errChan:
		r.Logger.Error().Err(err).Msg("relayer service failure")
		r.Stop()
		return err
	}
}

func (r *Relayer) Stop() {
	r.wg.Wait()
	r.Logger.Info().Msg("stopping relayer...")
}

// Fetch and execute Jobs
func (r *Relayer) processBroadcastedJobs(ctx context.Context) {
	defer r.wg.Done()
	r.Logger.Info().Msg("starting relayer worker...")

	for {
		select {
		case <-ctx.Done():
			return
		default:
			if r.broadcastQueue.Size() >= 1 {
				job, err := r.broadcastQueue.Pop()
				if err != nil {
					r.Logger.Error().Msgf("error fetching latest job from broadcast queue: %v", err)
					continue
				}
				txHash, err := r.EthClient.ExecuteCallback(ctx, job)
				if err != nil {
					r.Logger.Error().Msgf("error executing eth callback: %v", err)
					if _, ok := err.(*eth.FatalClientError); ok {
						r.errChan <- err
						return
					}
					continue
				}
				r.Logger.Info().Msg("successfully executed eth callback")
				if err = updateJobTxHash(r.db, job, txHash); err != nil {
					r.Logger.Error().Err(err).Msg("error updating job with transaction hash")
				}
			}
		}
	}
}

func updateJobTxHash(db db.DB, job *types.Job, txHash []byte) error {
	idBz := make([]byte, 4)
	binary.BigEndian.PutUint32(idBz, job.Id)

	job.TransactionHash = txHash

	bz, err := proto.Marshal(job)
	if err != nil {
		return fmt.Errorf("failed to marshal job: %w", err)
	}

	return db.Set(idBz, bz)
}
