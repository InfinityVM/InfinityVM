package relayer

import (
	"github.com/rs/zerolog"

	"github.com/ethos-works/InfinityVM/server/pkg/eth"
)

type Config struct {
	Logger          zerolog.Logger
	WorkerPoolCount int
}

type Manager struct {
	Config       *Config
	QueueService interface{}
	EthClient    *eth.EthClient
}

// Returns a new Job Manager
func NewManager(c *Config, qs interface{}, ec *eth.EthClient) *Manager {
	return &Manager{
		Config:       c,
		QueueService: qs,
		EthClient:    ec,
	}
}

// Starts the Manager Service
func (m *Manager) Start() error {
	panic("not implemented!")
}

// Stops the Manager Service
func (m *Manager) Stop() {
	panic("not implemented!")
}

// Fetch executed Jobs
// nolint:unused
func (m *Manager) fetchBroadcastedJobs() {
	panic("not implemented!")
}
