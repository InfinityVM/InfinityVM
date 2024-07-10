package server

import (
	"context"

	"github.com/ethos-works/InfinityVM/server/pkg/executor"
	"github.com/ethos-works/InfinityVM/server/pkg/types"
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

func (s *Server) SubmitProgram(context.Context, *types.SubmitProgramRequest) (*types.SubmitProgramResponse, error) {
	panic("not implemented!")
}
