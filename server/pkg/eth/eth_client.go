package eth

import (
	"context"
	"crypto/ecdsa"
	"encoding/hex"
	"fmt"
	"math/big"

	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/ethclient"
	"github.com/rs/zerolog"
	"github.com/rs/zerolog/log"

	jm "github.com/ethos-works/InfinityVM/server/pkg/eth/bindings/JobManager"
	"github.com/ethos-works/InfinityVM/server/pkg/types"
)

type EthClientI interface {
	ExecuteCallback(ctx context.Context, job *types.Job) error
}

type EthClient struct {
	client          *ethclient.Client
	signer          *bind.TransactOpts
	contractService *jm.ContractJobManagerTransactor
	log             zerolog.Logger
}

// Returns a new EthClient
func NewEthClient(ctx context.Context, log zerolog.Logger, ethHttpUrl, pk string, jobManagerAddress common.Address) (*EthClient, error) {
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

	contract, err := jm.NewContractJobManagerTransactor(jobManagerAddress, client)
	if err != nil {
		return nil, &FatalClientError{fmt.Sprintf("unable to initialize contract instance: %v", err)}
	}

	return &EthClient{
		client:          client,
		signer:          signer,
		contractService: contract,
		log:             log,
	}, nil
}

// Executes sequence to build and submit the submitResult transaction to the JobManager contract
func (c *EthClient) ExecuteCallback(ctx context.Context, job *types.Job) error {
	gasPrice, err := c.client.SuggestGasPrice(ctx)
	if err != nil {
		log.Error().Str("error", err.Error()).Msg("failed to retrieve gas price")
	}
	c.signer.GasPrice = gasPrice

	// TODO: Do we want to update the job record with the tx hash?
	tx, err := c.contractService.SubmitResult(c.signer, job.Result, job.ZkvmOperatorSignature)
	if err != nil {
		return fmt.Errorf("error submitting result to JobManager contract: %v", err)
	}
	log.Info().Str("tx", hex.EncodeToString(tx.Hash().Bytes())).Msg("successfully submitted result to JobManager contract")
	return nil
}
