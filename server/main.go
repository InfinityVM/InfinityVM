package main

import (
	"context"
	"fmt"
	"net"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/ethos-works/InfinityVM/server/pkg/server"
	"github.com/ethos-works/InfinityVM/server/pkg/types"
	"github.com/grpc-ecosystem/grpc-gateway/v2/runtime"
	"github.com/rs/zerolog"
	"golang.org/x/sync/errgroup"
	"google.golang.org/grpc"
)

func main() {
	if err := execute(); err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}

func execute() error {
	// TODO: Use cobra for CLI and wiring. This will allow us to get all relevant
	// values such as listen addresses and logger configuration.
	//
	// Ref: ETH-376

	ctx, cancel := context.WithCancel(context.Background())
	g, ctx := errgroup.WithContext(ctx)

	logger := zerolog.New(os.Stdout).With().Timestamp().Logger()

	// listen for and trap any OS signal to gracefully shutdown and exit
	trapSignal(cancel, logger)

	g.Go(func() error {
		return startGRPCServer(ctx, logger, "tcp", ":50051")
	})

	g.Go(func() error {
		return startGRPCGateway(ctx, logger, ":8080")
	})

	// Block main process until all spawned goroutines have gracefully exited and
	// signal has been captured in the main process or if an error occurs.
	return g.Wait()
}

// startGRPCServer starts a gRPC server and listens for incoming requests in a
// blocking process. It returns an error if the server cannot start.
func startGRPCServer(ctx context.Context, logger zerolog.Logger, network, listenAddr string, opts ...grpc.ServerOption) error {
	l, err := net.Listen(network, listenAddr)
	if err != nil {
		return err
	}

	defer func() {
		_ = l.Close()
	}()

	srv := grpc.NewServer(opts...)
	types.RegisterServiceServer(srv, server.New())

	srvErrCh := make(chan error, 1)

	go func() {
		logger.Info().Str("listen_addr", listenAddr).Msg("starting gRPC server...")
		srvErrCh <- srv.Serve(l)
	}()

	for {
		select {
		case <-ctx.Done():
			logger.Info().Str("listen_addr", listenAddr).Msg("shutting down gRPC server...")
			srv.GracefulStop()

			return nil

		case err := <-srvErrCh:
			logger.Error().Err(err).Msg("failed to start gRPC gateway server")
			return err
		}
	}
}

func startGRPCGateway(ctx context.Context, logger zerolog.Logger, listenAddr string, opts ...runtime.ServeMuxOption) error {
	mux := runtime.NewServeMux(opts...)
	types.RegisterServiceHandlerServer(ctx, mux, server.New())

	srvErrCh := make(chan error, 1)
	srv := &http.Server{
		Addr:    listenAddr,
		Handler: mux,
	}

	go func() {
		logger.Info().Str("listen_addr", listenAddr).Msg("starting gRPC gateway server...")
		srvErrCh <- srv.ListenAndServe()
	}()

	for {
		select {
		case <-ctx.Done():
			shutdownCtx, cancel := context.WithTimeout(ctx, 15*time.Second)
			defer cancel()

			logger.Info().Str("listen_addr", listenAddr).Msg("shutting down gRPC gateway server...")
			if err := srv.Shutdown(shutdownCtx); err != nil {
				logger.Error().Err(err).Msg("failed to gracefully shutdown gRPC gateway server")
				return err
			}

			return nil

		case err := <-srvErrCh:
			logger.Error().Err(err).Msg("failed to start gRPC gateway server")
			return err
		}
	}
}

// trapSignal will listen for any OS signal and invoke Done on the main WaitGroup
// allowing the main process to gracefully exit.
func trapSignal(cancel context.CancelFunc, logger zerolog.Logger) {
	sigCh := make(chan os.Signal, 1)

	signal.Notify(sigCh, syscall.SIGTERM)
	signal.Notify(sigCh, syscall.SIGINT)

	go func() {
		sig := <-sigCh
		logger.Info().Str("signal", sig.String()).Msg("caught signal; shutting down...")
		cancel()
	}()
}
