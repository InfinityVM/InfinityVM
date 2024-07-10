package cmd

import (
	"context"
	"fmt"
	"io"
	"net"
	"net/http"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	"github.com/grpc-ecosystem/grpc-gateway/v2/runtime"
	"github.com/rs/zerolog"
	"github.com/spf13/cobra"
	"golang.org/x/sync/errgroup"
	"google.golang.org/grpc"

	"github.com/ethos-works/InfinityVM/server/pkg/eth"
	"github.com/ethos-works/InfinityVM/server/pkg/executor"
	"github.com/ethos-works/InfinityVM/server/pkg/queue"
	"github.com/ethos-works/InfinityVM/server/pkg/relayer"
	"github.com/ethos-works/InfinityVM/server/pkg/server"
	"github.com/ethos-works/InfinityVM/server/pkg/types"
)

// CLI flag and value constants
const (
	logLevelJSON = "json"
	logLevelText = "text"

	flagLogLevel            = "log-level"
	flagLogFormat           = "log-format"
	flagGRPCEndpoint        = "grpc-endpoint"
	flagGRPCGatewayEndpoint = "grpc-gateway-endpoint"
	flagWorkerPool          = "worker-pool-count"
	flagZKShimAddress       = "zk-shim-address"
)

// RootCmd is the root command for the server CLI. All commands stem from the root
// command.
var RootCmd = &cobra.Command{
	Use:   "infinity-server",
	Short: "infinity-server is a gRPC server that runs the InfinityVM async enshrined coprocessing service",
	Long: `A gRPC server that runs the InfinityVM async enshrined coprocessing service.
The server is responsible for accepting and listening for new job execution requests
from clients and smart contracts. It will push jobs to be executed onto a queue
which are then fed into a zkVM shim process for execution and signature generation.
Completed jobs are then executed against the corresponding smart contract on InfinityVM.`,
	RunE: rootCmdHandler,
}

func init() {
	RootCmd.PersistentFlags().String(flagLogLevel, zerolog.InfoLevel.String(), "logging level")
	RootCmd.PersistentFlags().String(flagLogFormat, logLevelText, "logging format [json|text]")
	RootCmd.Flags().String(flagGRPCEndpoint, "localhost:50051", "The gRPC server endpoint")
	RootCmd.Flags().String(flagGRPCGatewayEndpoint, "localhost:8080", "The gRPC gateway server endpoint")
	RootCmd.Flags().String(flagZKShimAddress, "", "The ZK shim endpoint")

	RootCmd.AddCommand(getVersionCmd())
}

func rootCmdHandler(cmd *cobra.Command, args []string) error {
	logLvlStr, err := cmd.Flags().GetString(flagLogLevel)
	if err != nil {
		return err
	}

	logLvl, err := zerolog.ParseLevel(logLvlStr)
	if err != nil {
		return err
	}

	logFormatStr, err := cmd.Flags().GetString(flagLogFormat)
	if err != nil {
		return err
	}

	var logWriter io.Writer
	switch strings.ToLower(logFormatStr) {
	case logLevelJSON:
		logWriter = os.Stderr

	case logLevelText:
		logWriter = zerolog.ConsoleWriter{Out: os.Stderr}

	default:
		return fmt.Errorf("invalid logging format: %s", logFormatStr)
	}

	logger := zerolog.New(logWriter).Level(logLvl).With().Timestamp().Logger()

	ctx, cancel := context.WithCancel(context.Background())
	g, ctx := errgroup.WithContext(ctx)

	defer cancel()

	gRPCEndpoint, err := cmd.Flags().GetString(flagGRPCEndpoint)
	if err != nil {
		return err
	}

	gRPCGatewayEndpoint, err := cmd.Flags().GetString(flagGRPCGatewayEndpoint)
	if err != nil {
		return err
	}

	zkShimAddress, err := cmd.Flags().GetString(flagZKShimAddress)
	if err != nil {
		return err
	}

	workerCount, err := cmd.Flags().GetInt(flagWorkerPool)
	if err != nil {
		return err
	}

	execQueue := queue.NewMemQueue[*types.Job](executor.DefaultQueueSize)
	broadcastQueue := queue.NewMemQueue[*types.Job](executor.DefaultQueueSize)

	executor, err := executor.New(logger, nil, zkShimAddress, execQueue, broadcastQueue)
	if err != nil {
		return fmt.Errorf("failed to create executor: %w", err)
	}

	gRPCServer := server.New(executor)

	// listen for and trap any OS signal to gracefully shutdown and exit
	trapSignal(cancel, logger)

	g.Go(func() error {
		return startGRPCServer(ctx, logger, "tcp", gRPCEndpoint, gRPCServer)
	})

	g.Go(func() error {
		return startGRPCGateway(ctx, logger, gRPCGatewayEndpoint, gRPCServer)
	})

	g.Go(func() error {
		return startRelayer(ctx, logger, broadcastQueue, workerCount)
	})

	// Block main process until all spawned goroutines have gracefully exited and
	// signal has been captured in the main process or if an error occurs.
	return g.Wait()
}

// startGRPCServer starts a gRPC server and listens for incoming requests in a
// blocking process. It returns an error if the server cannot start.
func startGRPCServer(ctx context.Context, logger zerolog.Logger, network, listenAddr string, gRPCServer *server.Server, opts ...grpc.ServerOption) error {
	l, err := net.Listen(network, listenAddr)
	if err != nil {
		return err
	}

	defer func() {
		_ = l.Close()
	}()

	srv := grpc.NewServer(opts...)
	types.RegisterServiceServer(srv, gRPCServer)

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

func startGRPCGateway(ctx context.Context, logger zerolog.Logger, listenAddr string, gRPCServer *server.Server, opts ...runtime.ServeMuxOption) error {
	mux := runtime.NewServeMux(opts...)

	if err := types.RegisterServiceHandlerServer(ctx, mux, gRPCServer); err != nil {
		return fmt.Errorf("failed to register gRPC gateway server: %w", err)
	}

	srvErrCh := make(chan error, 1)
	srv := &http.Server{
		Addr:              listenAddr,
		Handler:           mux,
		ReadHeaderTimeout: 2 * time.Second,
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

// TODO: Determine if we need to inject EthClient
func startRelayer(ctx context.Context, logger zerolog.Logger, queue queue.Queue[*types.Job], workerCount int) error {
	// Configure Eth Client
	ethClient, err := eth.NewEthClient()
	if err != nil {
		return err
	}

	r := relayer.NewRelayer(logger, queue, ethClient, workerCount)

	if err := r.Start(ctx); err != nil {
		return fmt.Errorf("failed to start relayer: %w", err)
	}

	return nil
}

// trapSignal will listen for any OS signal and invoke Done on the main WaitGroup
// allowing the main process to gracefully exit.
func trapSignal(cancel context.CancelFunc, logger zerolog.Logger) {
	sigCh := make(chan os.Signal, 1)

	signal.Notify(sigCh, syscall.SIGTERM, syscall.SIGINT)

	go func() {
		sig := <-sigCh
		logger.Info().Str("signal", sig.String()).Msg("caught signal; shutting down...")
		cancel()
	}()
}
