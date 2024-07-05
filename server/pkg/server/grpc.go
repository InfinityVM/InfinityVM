package server

import (
	"context"

	"github.com/ethos-works/InfinityVM/server/pkg/types"
)

// TODO: Implement handlers!
//
// Ref: ETH-377

var _ types.ServiceServer = &Server{}

type Server struct {
	types.UnimplementedServiceServer
}

func New() *Server {
	return &Server{}
}

func (s *Server) SubmitJob(context.Context, *types.SubmitJobRequest) (*types.SubmitJobResponse, error) {
	panic("not implemented!")
}

func (s *Server) GetResult(context.Context, *types.GetResultRequest) (*types.GetResultResponse, error) {
	panic("not implemented!")
}

func (s *Server) CreateProgram(context.Context, *types.CreateProgramRequest) (*types.CreateProgramResponse, error) {
	panic("not implemented!")
}
