package eth

import (
	"context"
	"crypto/ecdsa"
	"fmt"
	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/ethclient"
	jm "github.com/ethos-works/InfinityVM/server/pkg/eth/bindings/JobManager"
	"github.com/ethos-works/InfinityVM/server/pkg/types"
	"github.com/rs/zerolog"
	"github.com/rs/zerolog/log"
	"math/big"
	"strings"
)

type EthClient interface {
	ExecuteCallback(job interface{}) error
}

type InfEthClient struct {
	signer          *bind.TransactOpts
	contractService *jm.ContractJobManagerTransactor
	log             zerolog.Logger
}

// Returns a new EthClient
// TODO: Configure contract address, signing key, etc
func NewEthClient(ctx context.Context, log zerolog.Logger, ethHttpUrl string, pk string, address common.Address) (*InfEthClient, error) {
	client, err := ethclient.Dial(ethHttpUrl)
	if err != nil {
		return nil, &FatalClientError{fmt.Sprintf("failed to create ETH client: %v", err)}
	}

	privateKey, err := crypto.HexToECDSA(pk)
	if err != nil {
		return nil, &FatalClientError{fmt.Sprintf("failed to cast privateKey to type *ecdsa.PrivateKey: %v", err)}
	}

	publicKey := privateKey.Public()
	publicKeyECDSA, ok := publicKey.(*ecdsa.PublicKey)
	if !ok {
		return nil, &FatalClientError{fmt.Sprintf("failed to cast publicKey to type *ecdsa.PublicKey: %v", err)}
	}

	fromAddress := crypto.PubkeyToAddress(*publicKeyECDSA)
	nonce, err := client.PendingNonceAt(ctx, fromAddress)
	if err != nil {
		return nil, &FatalClientError{fmt.Sprintf("failed to retrieve nonce: %v", err)}
	}

	gasPrice, err := client.SuggestGasPrice(ctx)
	if err != nil {
		return nil, &FatalClientError{fmt.Sprintf("failed to retrieve gas price: %v", err)}
	}

	chainID, err := client.NetworkID(ctx)
	if err != nil {
		return nil, &FatalClientError{fmt.Sprintf("failed to retrieve chain Id: %v", err)}
	}

	signer, err := bind.NewKeyedTransactorWithChainID(privateKey, chainID)
	if err != nil {
		return nil, &FatalClientError{fmt.Sprintf("unable to create signer: %v", err)}
	}
	signer.Nonce = big.NewInt(int64(nonce))
	signer.Value = big.NewInt(0)
	signer.GasLimit = uint64(3000000)
	signer.GasPrice = gasPrice

	contract, err := jm.NewContractJobManagerTransactor(address, client)
	if err != nil {
		return nil, &FatalClientError{fmt.Sprintf("unable to initialize contract instance: %v", err)}
	}

	return &InfEthClient{
		signer:          signer,
		contractService: contract,
		log:             log,
	}, nil
}

// Executes sequence to build and submit the submitResult transaction to the JobManager contract
func (c *InfEthClient) ExecuteCallback(job *types.Job, zkOpSig []byte) error {
	// TODO: need to encode result to []bytes

	result := &types.ResultMetadata{
		Result:           job.Result,
		JobId:            job.Id,
		ProgramId:        job.ProgramVerifyingKey,
		ProgramInputHash: job.Input,
	}
	bz, err := ecodeResultMetadata(result)
	if err != nil {
		return err
	}

	return c.submitResult(bz, zkOpSig)
}

// Call JobManager Contract with signed Result
// nolint:unused
func (c *InfEthClient) submitResult(result []byte, sig []byte) error {
	tx, err := c.contractService.SubmitResult(c.signer, result, sig)
	if err != nil {
		return fmt.Errorf("error submitting result to JobManager contract: %v", err)
	}
	log.Info().Bytes("tx", tx.Hash().Bytes()).Msg("successfully submitted result to JobManager contract")
	return nil
}

// Fetch inputs for a given Job
// nolint:unused
func (c *InfEthClient) getJobInputs() {
	panic("not implemented!")
}

func ecodeResultMetadata(result *types.ResultMetadata) ([]byte, error) {
	parsedABI, err := abi.JSON(strings.NewReader(jm.ContractJobManagerMetaData.ABI))
	if err != nil {
		return nil, fmt.Errorf("failed to parse ABI: %v", err)
	}
	bz, err := parsedABI.Pack("ResultMetadata", result.Result, result.JobId, result.ProgramId, result.ProgramInputHash)
	if err != nil {
		return nil, fmt.Errorf("fail")
	}

	return bz, nil
}
