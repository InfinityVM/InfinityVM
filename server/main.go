package main

import (
	"context"
	"net"

	"github.com/ethos-works/InfinityVM/server/pkg/server"
	"github.com/ethos-works/InfinityVM/server/pkg/types"

	"github.com/rs/zerolog"
	"google.golang.org/grpc"
)

func main() {
}

// startGRPCServer starts a gRPC server and listens for incoming requests in a
// blocking process. It returns an error if the server cannot start.
func startGRPCServer(ctx context.Context, logger zerolog.Logger, network, address string) error {
	l, err := net.Listen(network, address)
	if err != nil {
		return err
	}

	defer func() {
		if err := l.Close(); err != nil {
			logger.Error().Err(err).Str("address", address).Str("network", network).Msg("failed to close gRPC server")
		}
	}()

	svr := grpc.NewServer()
	types.RegisterServiceServer(svr, server.New())

	go func() {
		defer svr.GracefulStop()
		<-ctx.Done()
	}()

	return svr.Serve(l)
}

// var (
//   // command-line options:
//   // gRPC server endpoint
//   grpcServerEndpoint = flag.String("grpc-server-endpoint",  "localhost:9090", "gRPC server endpoint")
// )

// func run() error {
//   ctx := context.Background()
//   ctx, cancel := context.WithCancel(ctx)
//   defer cancel()

//   // Register gRPC server endpoint
//   // Note: Make sure the gRPC server is running properly and accessible
//   mux := runtime.NewServeMux()
//   opts := []grpc.DialOption{grpc.WithTransportCredentials(insecure.NewCredentials())}
//   // err := gw.RegisterYourServiceHandlerFromEndpoint(ctx, mux,  *grpcServerEndpoint, opts)
//   // if err != nil {
//   //   return err
//   // }

//   // Start HTTP server (and proxy calls to gRPC server endpoint)
//   return http.ListenAndServe(":8081", mux)
// }

// func main() {
//   flag.Parse()

//   if err := run(); err != nil {
//     grpclog.Fatal(err)
//   }
// }
