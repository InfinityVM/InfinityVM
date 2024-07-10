package server

import (
	"context"

	"github.com/ethos-works/InfinityVM/server/pkg/executor"
	"github.com/ethos-works/InfinityVM/server/pkg/types"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

var _ types.ServiceServer = &Server{}

type Server struct {
	types.UnimplementedServiceServer

	executor *executor.Executor
}

func New(e *executor.Executor) *Server {
	return &Server{
		executor: e,
	}
}

func (s *Server) SubmitJob(context.Context, *types.SubmitJobRequest) (*types.SubmitJobResponse, error) {
	panic("not implemented!")
}

func (s *Server) GetResult(context.Context, *types.GetResultRequest) (*types.GetResultResponse, error) {
	panic("not implemented!")
}

func (s *Server) SubmitProgram(ctx context.Context, req *types.SubmitProgramRequest) (*types.SubmitProgramResponse, error) {
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
