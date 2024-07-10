// Code generated - DO NOT EDIT.
// This file is a generated binding and any manual changes will be lost.

package contractConsumer

import (
	"errors"
	"math/big"
	"strings"

	ethereum "github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/event"
)

// Reference imports to suppress errors if they are not otherwise used.
var (
	_ = errors.New
	_ = big.NewInt
	_ = strings.NewReader
	_ = ethereum.NotFound
	_ = bind.Bind
	_ = common.Big1
	_ = types.BloomLookup
	_ = event.NewSubscription
	_ = abi.ConvertType
)

// ContractConsumerMetaData contains all meta data concerning the ContractConsumer contract.
var ContractConsumerMetaData = &bind.MetaData{
	ABI: "[{\"type\":\"function\",\"name\":\"getProgramInputsForJob\",\"inputs\":[{\"name\":\"jobID\",\"type\":\"uint32\",\"internalType\":\"uint32\"}],\"outputs\":[{\"name\":\"\",\"type\":\"bytes\",\"internalType\":\"bytes\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"receiveResult\",\"inputs\":[{\"name\":\"jobID\",\"type\":\"uint32\",\"internalType\":\"uint32\"},{\"name\":\"result\",\"type\":\"bytes\",\"internalType\":\"bytes\"}],\"outputs\":[],\"stateMutability\":\"nonpayable\"}]",
}

// ContractConsumerABI is the input ABI used to generate the binding from.
// Deprecated: Use ContractConsumerMetaData.ABI instead.
var ContractConsumerABI = ContractConsumerMetaData.ABI

// ContractConsumer is an auto generated Go binding around an Ethereum contract.
type ContractConsumer struct {
	ContractConsumerCaller     // Read-only binding to the contract
	ContractConsumerTransactor // Write-only binding to the contract
	ContractConsumerFilterer   // Log filterer for contract events
}

// ContractConsumerCaller is an auto generated read-only Go binding around an Ethereum contract.
type ContractConsumerCaller struct {
	contract *bind.BoundContract // Generic contract wrapper for the low level calls
}

// ContractConsumerTransactor is an auto generated write-only Go binding around an Ethereum contract.
type ContractConsumerTransactor struct {
	contract *bind.BoundContract // Generic contract wrapper for the low level calls
}

// ContractConsumerFilterer is an auto generated log filtering Go binding around an Ethereum contract events.
type ContractConsumerFilterer struct {
	contract *bind.BoundContract // Generic contract wrapper for the low level calls
}

// ContractConsumerSession is an auto generated Go binding around an Ethereum contract,
// with pre-set call and transact options.
type ContractConsumerSession struct {
	Contract     *ContractConsumer // Generic contract binding to set the session for
	CallOpts     bind.CallOpts     // Call options to use throughout this session
	TransactOpts bind.TransactOpts // Transaction auth options to use throughout this session
}

// ContractConsumerCallerSession is an auto generated read-only Go binding around an Ethereum contract,
// with pre-set call options.
type ContractConsumerCallerSession struct {
	Contract *ContractConsumerCaller // Generic contract caller binding to set the session for
	CallOpts bind.CallOpts           // Call options to use throughout this session
}

// ContractConsumerTransactorSession is an auto generated write-only Go binding around an Ethereum contract,
// with pre-set transact options.
type ContractConsumerTransactorSession struct {
	Contract     *ContractConsumerTransactor // Generic contract transactor binding to set the session for
	TransactOpts bind.TransactOpts           // Transaction auth options to use throughout this session
}

// ContractConsumerRaw is an auto generated low-level Go binding around an Ethereum contract.
type ContractConsumerRaw struct {
	Contract *ContractConsumer // Generic contract binding to access the raw methods on
}

// ContractConsumerCallerRaw is an auto generated low-level read-only Go binding around an Ethereum contract.
type ContractConsumerCallerRaw struct {
	Contract *ContractConsumerCaller // Generic read-only contract binding to access the raw methods on
}

// ContractConsumerTransactorRaw is an auto generated low-level write-only Go binding around an Ethereum contract.
type ContractConsumerTransactorRaw struct {
	Contract *ContractConsumerTransactor // Generic write-only contract binding to access the raw methods on
}

// NewContractConsumer creates a new instance of ContractConsumer, bound to a specific deployed contract.
func NewContractConsumer(address common.Address, backend bind.ContractBackend) (*ContractConsumer, error) {
	contract, err := bindContractConsumer(address, backend, backend, backend)
	if err != nil {
		return nil, err
	}
	return &ContractConsumer{ContractConsumerCaller: ContractConsumerCaller{contract: contract}, ContractConsumerTransactor: ContractConsumerTransactor{contract: contract}, ContractConsumerFilterer: ContractConsumerFilterer{contract: contract}}, nil
}

// NewContractConsumerCaller creates a new read-only instance of ContractConsumer, bound to a specific deployed contract.
func NewContractConsumerCaller(address common.Address, caller bind.ContractCaller) (*ContractConsumerCaller, error) {
	contract, err := bindContractConsumer(address, caller, nil, nil)
	if err != nil {
		return nil, err
	}
	return &ContractConsumerCaller{contract: contract}, nil
}

// NewContractConsumerTransactor creates a new write-only instance of ContractConsumer, bound to a specific deployed contract.
func NewContractConsumerTransactor(address common.Address, transactor bind.ContractTransactor) (*ContractConsumerTransactor, error) {
	contract, err := bindContractConsumer(address, nil, transactor, nil)
	if err != nil {
		return nil, err
	}
	return &ContractConsumerTransactor{contract: contract}, nil
}

// NewContractConsumerFilterer creates a new log filterer instance of ContractConsumer, bound to a specific deployed contract.
func NewContractConsumerFilterer(address common.Address, filterer bind.ContractFilterer) (*ContractConsumerFilterer, error) {
	contract, err := bindContractConsumer(address, nil, nil, filterer)
	if err != nil {
		return nil, err
	}
	return &ContractConsumerFilterer{contract: contract}, nil
}

// bindContractConsumer binds a generic wrapper to an already deployed contract.
func bindContractConsumer(address common.Address, caller bind.ContractCaller, transactor bind.ContractTransactor, filterer bind.ContractFilterer) (*bind.BoundContract, error) {
	parsed, err := ContractConsumerMetaData.GetAbi()
	if err != nil {
		return nil, err
	}
	return bind.NewBoundContract(address, *parsed, caller, transactor, filterer), nil
}

// Call invokes the (constant) contract method with params as input values and
// sets the output to result. The result type might be a single field for simple
// returns, a slice of interfaces for anonymous returns and a struct for named
// returns.
func (_ContractConsumer *ContractConsumerRaw) Call(opts *bind.CallOpts, result *[]interface{}, method string, params ...interface{}) error {
	return _ContractConsumer.Contract.ContractConsumerCaller.contract.Call(opts, result, method, params...)
}

// Transfer initiates a plain transaction to move funds to the contract, calling
// its default method if one is available.
func (_ContractConsumer *ContractConsumerRaw) Transfer(opts *bind.TransactOpts) (*types.Transaction, error) {
	return _ContractConsumer.Contract.ContractConsumerTransactor.contract.Transfer(opts)
}

// Transact invokes the (paid) contract method with params as input values.
func (_ContractConsumer *ContractConsumerRaw) Transact(opts *bind.TransactOpts, method string, params ...interface{}) (*types.Transaction, error) {
	return _ContractConsumer.Contract.ContractConsumerTransactor.contract.Transact(opts, method, params...)
}

// Call invokes the (constant) contract method with params as input values and
// sets the output to result. The result type might be a single field for simple
// returns, a slice of interfaces for anonymous returns and a struct for named
// returns.
func (_ContractConsumer *ContractConsumerCallerRaw) Call(opts *bind.CallOpts, result *[]interface{}, method string, params ...interface{}) error {
	return _ContractConsumer.Contract.contract.Call(opts, result, method, params...)
}

// Transfer initiates a plain transaction to move funds to the contract, calling
// its default method if one is available.
func (_ContractConsumer *ContractConsumerTransactorRaw) Transfer(opts *bind.TransactOpts) (*types.Transaction, error) {
	return _ContractConsumer.Contract.contract.Transfer(opts)
}

// Transact invokes the (paid) contract method with params as input values.
func (_ContractConsumer *ContractConsumerTransactorRaw) Transact(opts *bind.TransactOpts, method string, params ...interface{}) (*types.Transaction, error) {
	return _ContractConsumer.Contract.contract.Transact(opts, method, params...)
}

// GetProgramInputsForJob is a free data retrieval call binding the contract method 0x76a5c409.
//
// Solidity: function getProgramInputsForJob(uint32 jobID) view returns(bytes)
func (_ContractConsumer *ContractConsumerCaller) GetProgramInputsForJob(opts *bind.CallOpts, jobID uint32) ([]byte, error) {
	var out []interface{}
	err := _ContractConsumer.contract.Call(opts, &out, "getProgramInputsForJob", jobID)

	if err != nil {
		return *new([]byte), err
	}

	out0 := *abi.ConvertType(out[0], new([]byte)).(*[]byte)

	return out0, err

}

// GetProgramInputsForJob is a free data retrieval call binding the contract method 0x76a5c409.
//
// Solidity: function getProgramInputsForJob(uint32 jobID) view returns(bytes)
func (_ContractConsumer *ContractConsumerSession) GetProgramInputsForJob(jobID uint32) ([]byte, error) {
	return _ContractConsumer.Contract.GetProgramInputsForJob(&_ContractConsumer.CallOpts, jobID)
}

// GetProgramInputsForJob is a free data retrieval call binding the contract method 0x76a5c409.
//
// Solidity: function getProgramInputsForJob(uint32 jobID) view returns(bytes)
func (_ContractConsumer *ContractConsumerCallerSession) GetProgramInputsForJob(jobID uint32) ([]byte, error) {
	return _ContractConsumer.Contract.GetProgramInputsForJob(&_ContractConsumer.CallOpts, jobID)
}

// ReceiveResult is a paid mutator transaction binding the contract method 0x457e284f.
//
// Solidity: function receiveResult(uint32 jobID, bytes result) returns()
func (_ContractConsumer *ContractConsumerTransactor) ReceiveResult(opts *bind.TransactOpts, jobID uint32, result []byte) (*types.Transaction, error) {
	return _ContractConsumer.contract.Transact(opts, "receiveResult", jobID, result)
}

// ReceiveResult is a paid mutator transaction binding the contract method 0x457e284f.
//
// Solidity: function receiveResult(uint32 jobID, bytes result) returns()
func (_ContractConsumer *ContractConsumerSession) ReceiveResult(jobID uint32, result []byte) (*types.Transaction, error) {
	return _ContractConsumer.Contract.ReceiveResult(&_ContractConsumer.TransactOpts, jobID, result)
}

// ReceiveResult is a paid mutator transaction binding the contract method 0x457e284f.
//
// Solidity: function receiveResult(uint32 jobID, bytes result) returns()
func (_ContractConsumer *ContractConsumerTransactorSession) ReceiveResult(jobID uint32, result []byte) (*types.Transaction, error) {
	return _ContractConsumer.Contract.ReceiveResult(&_ContractConsumer.TransactOpts, jobID, result)
}
