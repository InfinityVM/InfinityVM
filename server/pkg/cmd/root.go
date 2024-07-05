package cmd

import (
	"github.com/rs/zerolog"
	"github.com/spf13/cobra"
)

// CLI flag and value constants
const (
	logLevelJSON = "json"
	logLevelText = "text"

	flagLogLevel  = "log-level"
	flagLogFormat = "log-format"
)

var rootCmd = &cobra.Command{
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
	rootCmd.PersistentFlags().String(flagLogLevel, zerolog.InfoLevel.String(), "logging level")
	rootCmd.PersistentFlags().String(flagLogFormat, logLevelText, "logging format [json|text]")

	rootCmd.AddCommand(getVersionCmd())
}

func rootCmdHandler(cmd *cobra.Command, args []string) error {
	panic("not implemented!")
}
