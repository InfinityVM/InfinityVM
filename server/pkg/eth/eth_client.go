package eth

type EthClientI interface {
	ExecuteCallback(job interface{}) error
}

type EthClient struct{}

// Returns a new EthClientI
// TODO: Configure contract address, signing key, etc
func NewEthClient() (*EthClient, error) {
	return &EthClient{}, nil
}

// Executes sequence to build and submit the submitResult transaction to the JobManager contract
func (c *EthClient) ExecuteCallback(job interface{}) error {
	return nil
}

// Call JobManager Contract with signed Result
// nolint:unused
func (c *EthClient) submitResult() {
	panic("not implemented!")
}

// Fetch inputs for a given Job
// nolint:unused
func (c *EthClient) getJobInputs() {
	panic("not implemented!")
}
