package eth

type EthClient struct{}

// Returns a new EthClient
// TODO: Configure contract address, signing key, etc
func NewEthClient() (*EthClient, error) {
	return &EthClient{}, nil
}

// Executes sequence to build and submit the submitResult transaction to the JobManager contract
func (c *EthClient) ExecuteCallback() {
	panic("not implemented!")
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
