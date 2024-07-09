package eth

type EthClient interface {
	ExecuteCallback(job interface{}) error
}

type InfEthClient struct{}

// Returns a new EthClient
// TODO: Configure contract address, signing key, etc
func NewEthClient() (*InfEthClient, error) {
	return &InfEthClient{}, nil
}

// Executes sequence to build and submit the submitResult transaction to the JobManager contract
func (c *InfEthClient) ExecuteCallback(job interface{}) error {
	return nil
}

// Call JobManager Contract with signed Result
// nolint:unused
func (c *InfEthClient) submitResult() {
	panic("not implemented!")
}

// Fetch inputs for a given Job
// nolint:unused
func (c *InfEthClient) getJobInputs() {
	panic("not implemented!")
}
