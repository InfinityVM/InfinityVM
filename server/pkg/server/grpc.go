package server

import (
	"context"

	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"

	"github.com/ethos-works/InfinityVM/server/pkg/executor"
	"github.com/ethos-works/InfinityVM/server/pkg/types"
)

var _ types.ServiceServer = &Server{}

// Server implements the gRPC service for program execution and management.
type Server struct {
	types.UnimplementedServiceServer

	executor *executor.Executor
}

func New(e *executor.Executor) *Server {
	return &Server{
		executor: e,
	}
}

func (s *Server) SubmitJob(_ context.Context, req *types.SubmitJobRequest) (*types.SubmitJobResponse, error) {
	if req == nil {
		return nil, status.Errorf(codes.InvalidArgument, "empty request")
	}

	// verify fields
	if req.Job.MaxCycles == 0 {
		return nil, status.Errorf(codes.InvalidArgument, "max cycles must be positive")
	}
	if req.Job.Id == 0 {
		return nil, status.Errorf(codes.InvalidArgument, "contract ID must be positive")
	}
	if len(req.Job.ContractAddress) == 0 {
		return nil, status.Errorf(codes.InvalidArgument, "contract address must not be empty")
	}
	if len(req.Job.ProgramVerifyingKey) == 0 {
		return nil, status.Errorf(codes.InvalidArgument, "program verification key must not be empty")
	}

	if err := s.executor.SubmitJob(req.Job); err != nil {
		return nil, status.Errorf(codes.Internal, "failed to submit job: %v", err)
	}

	return &types.SubmitJobResponse{JobId: req.Job.Id}, nil
}

func (s *Server) GetResult(_ context.Context, req *types.GetResultRequest) (*types.GetResultResponse, error) {
	if req == nil {
		return nil, status.Errorf(codes.InvalidArgument, "empty request")
	}

	if req.JobId == 0 {
		return nil, status.Errorf(codes.InvalidArgument, "contract ID must be positive")
	}

	job, err := s.executor.GetJob(req.JobId)
	if err != nil {
		return nil, status.Errorf(codes.Internal, "failed to get job (%d): %v", req.JobId, err)
	}

	return &types.GetResultResponse{Job: job}, nil
}

func (s *Server) SubmitProgram(_ context.Context, req *types.SubmitProgramRequest) (*types.SubmitProgramResponse, error) {
	if req == nil {
		return nil, status.Errorf(codes.InvalidArgument, "empty request")
	}

	verificationKey, err := s.executor.SubmitELF(req.ProgramElf)
	if err != nil {
		return nil, status.Errorf(codes.Internal, "failed to get verification key: %v", err)
	}

	return &types.SubmitProgramResponse{
		ProgramVerifyingKey: verificationKey,
	}, nil
}
