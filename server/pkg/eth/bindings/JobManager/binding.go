// Code generated - DO NOT EDIT.
// This file is a generated binding and any manual changes will be lost.

package contractJobManager

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

// IJobManagerJobMetadata is an auto generated low-level Go binding around an user-defined struct.
type IJobManagerJobMetadata struct {
	ProgramID []byte
	Caller    common.Address
	Status    uint8
}

// ContractJobManagerMetaData contains all meta data concerning the ContractJobManager contract.
var ContractJobManagerMetaData = &bind.MetaData{
	ABI: "[{\"type\":\"constructor\",\"inputs\":[{\"name\":\"_relayer\",\"type\":\"address\",\"internalType\":\"address\"},{\"name\":\"_coprocessorOperator\",\"type\":\"address\",\"internalType\":\"address\"}],\"stateMutability\":\"nonpayable\"},{\"type\":\"function\",\"name\":\"cancelJob\",\"inputs\":[{\"name\":\"jobID\",\"type\":\"uint32\",\"internalType\":\"uint32\"}],\"outputs\":[],\"stateMutability\":\"nonpayable\"},{\"type\":\"function\",\"name\":\"createJob\",\"inputs\":[{\"name\":\"programID\",\"type\":\"bytes\",\"internalType\":\"bytes\"},{\"name\":\"programInput\",\"type\":\"bytes\",\"internalType\":\"bytes\"}],\"outputs\":[{\"name\":\"jobID\",\"type\":\"uint32\",\"internalType\":\"uint32\"}],\"stateMutability\":\"nonpayable\"},{\"type\":\"function\",\"name\":\"getJobMetadata\",\"inputs\":[{\"name\":\"jobID\",\"type\":\"uint32\",\"internalType\":\"uint32\"}],\"outputs\":[{\"name\":\"\",\"type\":\"tuple\",\"internalType\":\"structIJobManager.JobMetadata\",\"components\":[{\"name\":\"programID\",\"type\":\"bytes\",\"internalType\":\"bytes\"},{\"name\":\"caller\",\"type\":\"address\",\"internalType\":\"address\"},{\"name\":\"status\",\"type\":\"uint8\",\"internalType\":\"uint8\"}]}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"initializeJobManager\",\"inputs\":[{\"name\":\"initialOwner\",\"type\":\"address\",\"internalType\":\"address\"}],\"outputs\":[],\"stateMutability\":\"nonpayable\"},{\"type\":\"function\",\"name\":\"jobIDCounter\",\"inputs\":[],\"outputs\":[{\"name\":\"\",\"type\":\"uint32\",\"internalType\":\"uint32\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"jobIDToMetadata\",\"inputs\":[{\"name\":\"\",\"type\":\"uint32\",\"internalType\":\"uint32\"}],\"outputs\":[{\"name\":\"programID\",\"type\":\"bytes\",\"internalType\":\"bytes\"},{\"name\":\"caller\",\"type\":\"address\",\"internalType\":\"address\"},{\"name\":\"status\",\"type\":\"uint8\",\"internalType\":\"uint8\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"owner\",\"inputs\":[],\"outputs\":[{\"name\":\"\",\"type\":\"address\",\"internalType\":\"address\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"renounceOwnership\",\"inputs\":[],\"outputs\":[],\"stateMutability\":\"nonpayable\"},{\"type\":\"function\",\"name\":\"setCoprocessorOperator\",\"inputs\":[{\"name\":\"_coprocessorOperator\",\"type\":\"address\",\"internalType\":\"address\"}],\"outputs\":[],\"stateMutability\":\"nonpayable\"},{\"type\":\"function\",\"name\":\"setRelayer\",\"inputs\":[{\"name\":\"_relayer\",\"type\":\"address\",\"internalType\":\"address\"}],\"outputs\":[],\"stateMutability\":\"nonpayable\"},{\"type\":\"function\",\"name\":\"submitResult\",\"inputs\":[{\"name\":\"resultWithMetadata\",\"type\":\"bytes\",\"internalType\":\"bytes\"},{\"name\":\"signature\",\"type\":\"bytes\",\"internalType\":\"bytes\"}],\"outputs\":[],\"stateMutability\":\"nonpayable\"},{\"type\":\"function\",\"name\":\"transferOwnership\",\"inputs\":[{\"name\":\"newOwner\",\"type\":\"address\",\"internalType\":\"address\"}],\"outputs\":[],\"stateMutability\":\"nonpayable\"},{\"type\":\"event\",\"name\":\"Initialized\",\"inputs\":[{\"name\":\"version\",\"type\":\"uint64\",\"indexed\":false,\"internalType\":\"uint64\"}],\"anonymous\":false},{\"type\":\"event\",\"name\":\"JobCancelled\",\"inputs\":[{\"name\":\"jobID\",\"type\":\"uint32\",\"indexed\":true,\"internalType\":\"uint32\"}],\"anonymous\":false},{\"type\":\"event\",\"name\":\"JobCompleted\",\"inputs\":[{\"name\":\"jobID\",\"type\":\"uint32\",\"indexed\":true,\"internalType\":\"uint32\"},{\"name\":\"result\",\"type\":\"bytes\",\"indexed\":false,\"internalType\":\"bytes\"}],\"anonymous\":false},{\"type\":\"event\",\"name\":\"JobCreated\",\"inputs\":[{\"name\":\"jobID\",\"type\":\"uint32\",\"indexed\":true,\"internalType\":\"uint32\"},{\"name\":\"programID\",\"type\":\"bytes\",\"indexed\":false,\"internalType\":\"bytes\"},{\"name\":\"programInput\",\"type\":\"bytes\",\"indexed\":false,\"internalType\":\"bytes\"}],\"anonymous\":false},{\"type\":\"event\",\"name\":\"OwnershipTransferred\",\"inputs\":[{\"name\":\"previousOwner\",\"type\":\"address\",\"indexed\":true,\"internalType\":\"address\"},{\"name\":\"newOwner\",\"type\":\"address\",\"indexed\":true,\"internalType\":\"address\"}],\"anonymous\":false},{\"type\":\"error\",\"name\":\"InvalidInitialization\",\"inputs\":[]},{\"type\":\"error\",\"name\":\"NotInitializing\",\"inputs\":[]},{\"type\":\"error\",\"name\":\"OwnableInvalidOwner\",\"inputs\":[{\"name\":\"owner\",\"type\":\"address\",\"internalType\":\"address\"}]},{\"type\":\"error\",\"name\":\"OwnableUnauthorizedAccount\",\"inputs\":[{\"name\":\"account\",\"type\":\"address\",\"internalType\":\"address\"}]},{\"type\":\"error\",\"name\":\"ReentrancyGuardReentrantCall\",\"inputs\":[]}]",
	Bin: "0x60806040526001805463ffffffff19168117905534801561001f57600080fd5b5060405161185b38038061185b83398101604081905261003e91610164565b600160008190558054600160201b600160c01b0319166401000000006001600160a01b038581169190910291909117909155600280546001600160a01b03191691831691909117905561008f610096565b5050610197565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00805468010000000000000000900460ff16156100e65760405163f92ee8a960e01b815260040160405180910390fd5b80546001600160401b03908116146101455780546001600160401b0319166001600160401b0390811782556040519081527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b50565b80516001600160a01b038116811461015f57600080fd5b919050565b6000806040838503121561017757600080fd5b61018083610148565b915061018e60208401610148565b90509250929050565b6116b5806101a66000396000f3fe608060405234801561001057600080fd5b50600436106100b45760003560e01c80638eb533ef116100715780638eb533ef14610147578063b577a6e41461015a578063d7967c0714610182578063ea25348714610192578063f2fde38b146101a5578063f9918a80146101b857600080fd5b806333db8f79146100b957806362020438146100ce5780636548e9bc146100e1578063715018a6146100f457806371a7fd92146100fc5780638da5cb5b14610127575b600080fd5b6100cc6100c7366004611084565b6101d8565b005b6100cc6100dc366004611084565b610202565b6100cc6100ef366004611084565b610312565b6100cc610348565b61010f61010a3660046110cd565b61035c565b60405161011e93929190611138565b60405180910390f35b61012f610416565b6040516001600160a01b03909116815260200161011e565b6100cc6101553660046110cd565b610444565b61016d6101683660046111b7565b610673565b60405163ffffffff909116815260200161011e565b60015461016d9063ffffffff1681565b6100cc6101a03660046111b7565b6107aa565b6100cc6101b3366004611084565b610d8f565b6101cb6101c63660046110cd565b610dcd565b60405161011e9190611228565b6101e0610ec6565b600280546001600160a01b0319166001600160a01b0392909216919091179055565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a008054600160401b810460ff16159067ffffffffffffffff166000811580156102485750825b905060008267ffffffffffffffff1660011480156102655750303b155b905081158015610273575080155b156102915760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff1916600117855583156102bb57845460ff60401b1916600160401b1785555b6102c486610ef8565b831561030a57845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b505050505050565b61031a610ec6565b600180546001600160a01b0390921664010000000002640100000000600160c01b0319909216919091179055565b610350610ec6565b61035a6000610ef8565b565b60036020526000908152604090208054819061037790611272565b80601f01602080910402602001604051908101604052809291908181526020018280546103a390611272565b80156103f05780601f106103c5576101008083540402835291602001916103f0565b820191906000526020600020905b8154815290600101906020018083116103d357829003601f168201915b505050600190930154919250506001600160a01b0381169060ff600160a01b9091041683565b7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b031690565b63ffffffff811660009081526003602052604080822081516060810190925280548290829061047290611272565b80601f016020809104026020016040519081016040528092919081815260200182805461049e90611272565b80156104eb5780601f106104c0576101008083540402835291602001916104eb565b820191906000526020600020905b8154815290600101906020018083116104ce57829003601f168201915b5050509183525050600191909101546001600160a01b03808216602080850191909152600160a01b90920460ff166040909301929092528201519192501633148061054e5750610539610416565b6001600160a01b0316336001600160a01b0316145b6105d55760405162461bcd60e51b815260206004820152604760248201527f4a6f624d616e616765722e63616e63656c4a6f623a2063616c6c65722069732060448201527f6e6f7420746865206a6f622063726561746f72206f72204a6f624d616e616765606482015266391037bbb732b960c91b608482015260a4015b60405180910390fd5b600160408083019190915263ffffffff83166000908152600360205220815182919081906106039082611311565b5060208201516001909101805460409384015160ff16600160a01b026001600160a81b03199091166001600160a01b03909316929092179190911790555163ffffffff8316907fa6a2eb5c45aee3821a738c1beb0dade684d1189539bab9d4f634c00de2449e7a90600090a25050565b6001546040805160806020601f880181900402820181019092526060810186815263ffffffff90931692909182919088908890819085018382808284376000920182905250938552505033602080850191909152604093840183905263ffffffff86168352600390525020815181906106ec9082611311565b5060208201516001909101805460409384015160ff16600160a01b026001600160a81b03199091166001600160a01b03909316929092179190911790555163ffffffff8216907f889a0aa153aefbb0af2faa68e340f3dbeb9ff2b45c378ae4c707bbaf13e3a924906107659088908890889088906113f9565b60405180910390a26001805463ffffffff169060006107838361142b565b91906101000a81548163ffffffff021916908363ffffffff16021790555050949350505050565b6107b2610f69565b60015464010000000090046001600160a01b0316331461082f5760405162461bcd60e51b815260206004820152603260248201527f4a6f624d616e616765722e7375626d6974526573756c743a2063616c6c65722060448201527134b9903737ba103a3432903932b630bcb2b960711b60648201526084016105cc565b6040516000906108479085908790829060200161145e565b60405160208183030381529060405280519060200120905060006108a18285858080601f016020809104026020016040519081016040528093929190818152602001838380828437600092019190915250610f9392505050565b6002549091506001600160a01b038083169116146109145760405162461bcd60e51b815260206004820152602a60248201527f4a6f624d616e616765722e7375626d6974526573756c743a20496e76616c6964604482015269207369676e617475726560b01b60648201526084016105cc565b6000808080610925898b018b61154d565b93509350935093506000600360008563ffffffff1663ffffffff16815260200190815260200160002060405180606001604052908160008201805461096990611272565b80601f016020809104026020016040519081016040528092919081815260200182805461099590611272565b80156109e25780601f106109b7576101008083540402835291602001916109e2565b820191906000526020600020905b8154815290600101906020018083116109c557829003601f168201915b5050509183525050600191909101546001600160a01b038116602083015260ff600160a01b9091048116604092830152908201519192501615610a845760405162461bcd60e51b815260206004820152603460248201527f4a6f624d616e616765722e7375626d6974526573756c743a206a6f62206973206044820152736e6f7420696e2070656e64696e6720737461746560601b60648201526084016105cc565b828051906020012081600001518051906020012014610b3f5760405162461bcd60e51b815260206004820152606560248201527f4a6f624d616e616765722e7375626d6974526573756c743a2070726f6772616d60448201527f204944207369676e656420627920636f70726f636573736f7220646f65736e2760648201527f74206d617463682070726f6772616d204944207375626d69747465642077697460848201526434103537b160d91b60a482015260c4016105cc565b60208101516040516376a5c40960e01b815263ffffffff8616600482015283916001600160a01b0316906376a5c40990602401600060405180830381865afa158015610b8f573d6000803e3d6000fd5b505050506040513d6000823e601f3d908101601f19168201604052610bb791908101906115ce565b8051906020012014610c6b5760405162461bcd60e51b815260206004820152606b60248201527f4a6f624d616e616765722e7375626d6974526573756c743a2070726f6772616d60448201527f20696e707574207369676e656420627920636f70726f636573736f7220646f6560648201527f736e2774206d617463682070726f6772616d20696e707574207375626d69747460848201526a32b2103bb4ba34103537b160a91b60a482015260c4016105cc565b600260408083019190915263ffffffff8516600090815260036020522081518291908190610c999082611311565b5060208201516001909101805460409384015160ff16600160a01b026001600160a81b03199091166001600160a01b03909316929092179190911790555163ffffffff8516907fd75a350cc0ba7be9680891736117e26b5fdf4bff5685a8a9346af44d87022be590610d0c908890611645565b60405180910390a280602001516001600160a01b031663457e284f85876040518363ffffffff1660e01b8152600401610d46929190611658565b600060405180830381600087803b158015610d6057600080fd5b505af1158015610d74573d6000803e3d6000fd5b5050505050505050505050610d896001600055565b50505050565b610d97610ec6565b6001600160a01b038116610dc157604051631e4fbdf760e01b8152600060048201526024016105cc565b610dca81610ef8565b50565b604080516060808201835281526000602082018190529181019190915263ffffffff821660009081526003602052604090819020815160608101909252805482908290610e1990611272565b80601f0160208091040260200160405190810160405280929190818152602001828054610e4590611272565b8015610e925780601f10610e6757610100808354040283529160200191610e92565b820191906000526020600020905b815481529060010190602001808311610e7557829003601f168201915b5050509183525050600191909101546001600160a01b0381166020830152600160a01b900460ff1660409091015292915050565b33610ecf610416565b6001600160a01b03161461035a5760405163118cdaa760e01b81523360048201526024016105cc565b7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c19930080546001600160a01b031981166001600160a01b03848116918217845560405192169182907f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e090600090a3505050565b600260005403610f8c57604051633ee5aeb560e01b815260040160405180910390fd5b6002600055565b600080600080610fa285611012565b6040805160008152602081018083528b905260ff8316918101919091526060810184905260808101839052929550909350915060019060a0016020604051602081039080840390855afa158015610ffd573d6000803e3d6000fd5b5050604051601f190151979650505050505050565b600080600083516041146110685760405162461bcd60e51b815260206004820152601860248201527f696e76616c6964207369676e6174757265206c656e677468000000000000000060448201526064016105cc565b50505060208101516040820151606083015160001a9193909250565b60006020828403121561109657600080fd5b81356001600160a01b03811681146110ad57600080fd5b9392505050565b803563ffffffff811681146110c857600080fd5b919050565b6000602082840312156110df57600080fd5b6110ad826110b4565b60005b838110156111035781810151838201526020016110eb565b50506000910152565b600081518084526111248160208601602086016110e8565b601f01601f19169290920160200192915050565b60608152600061114b606083018661110c565b6001600160a01b039490941660208301525060ff91909116604090910152919050565b60008083601f84011261118057600080fd5b50813567ffffffffffffffff81111561119857600080fd5b6020830191508360208285010111156111b057600080fd5b9250929050565b600080600080604085870312156111cd57600080fd5b843567ffffffffffffffff8111156111e457600080fd5b6111f08782880161116e565b909550935050602085013567ffffffffffffffff81111561121057600080fd5b61121c8782880161116e565b95989497509550505050565b602081526000825160606020840152611244608084018261110c565b60208501516001600160a01b03166040858101919091529094015160ff166060909301929092525090919050565b600181811c9082168061128657607f821691505b6020821081036112a657634e487b7160e01b600052602260045260246000fd5b50919050565b634e487b7160e01b600052604160045260246000fd5b601f82111561130c57806000526020600020601f840160051c810160208510156112e95750805b601f840160051c820191505b8181101561130957600081556001016112f5565b50505b505050565b815167ffffffffffffffff81111561132b5761132b6112ac565b61133f816113398454611272565b846112c2565b6020601f821160018114611373576000831561135b5750848201515b600019600385901b1c1916600184901b178455611309565b600084815260208120601f198516915b828110156113a35787850151825560209485019460019092019101611383565b50848210156113c15786840151600019600387901b60f8161c191681555b50505050600190811b01905550565b81835281816020850137506000828201602090810191909152601f909101601f19169091010190565b60408152600061140d6040830186886113d0565b82810360208401526114208185876113d0565b979650505050505050565b600063ffffffff821663ffffffff810361145557634e487b7160e01b600052601160045260246000fd5b60010192915050565b7f19457468657265756d205369676e6564204d6573736167653a0a000000000000815283601a8201528183603a83013760009101603a0190815292915050565b604051601f8201601f1916810167ffffffffffffffff811182821017156114c7576114c76112ac565b604052919050565b600067ffffffffffffffff8211156114e9576114e96112ac565b50601f01601f191660200190565b600082601f83011261150857600080fd5b813561151b611516826114cf565b61149e565b81815284602083860101111561153057600080fd5b816020850160208301376000918101602001919091529392505050565b6000806000806080858703121561156357600080fd5b843567ffffffffffffffff81111561157a57600080fd5b611586878288016114f7565b945050611595602086016110b4565b9250604085013567ffffffffffffffff8111156115b157600080fd5b6115bd878288016114f7565b949793965093946060013593505050565b6000602082840312156115e057600080fd5b815167ffffffffffffffff8111156115f757600080fd5b8201601f8101841361160857600080fd5b8051611616611516826114cf565b81815285602083850101111561162b57600080fd5b61163c8260208301602086016110e8565b95945050505050565b6020815260006110ad602083018461110c565b63ffffffff83168152604060208201526000611677604083018461110c565b94935050505056fea26469706673582212207b0cee663f3f5ea33f581cc5104ab54077a7699e414774b6256c1bd602c3efe364736f6c634300081a0033",
}

// ContractJobManagerABI is the input ABI used to generate the binding from.
// Deprecated: Use ContractJobManagerMetaData.ABI instead.
var ContractJobManagerABI = ContractJobManagerMetaData.ABI

// ContractJobManagerBin is the compiled bytecode used for deploying new contracts.
// Deprecated: Use ContractJobManagerMetaData.Bin instead.
var ContractJobManagerBin = ContractJobManagerMetaData.Bin

// DeployContractJobManager deploys a new Ethereum contract, binding an instance of ContractJobManager to it.
func DeployContractJobManager(auth *bind.TransactOpts, backend bind.ContractBackend, _relayer common.Address, _coprocessorOperator common.Address) (common.Address, *types.Transaction, *ContractJobManager, error) {
	parsed, err := ContractJobManagerMetaData.GetAbi()
	if err != nil {
		return common.Address{}, nil, nil, err
	}
	if parsed == nil {
		return common.Address{}, nil, nil, errors.New("GetABI returned nil")
	}

	address, tx, contract, err := bind.DeployContract(auth, *parsed, common.FromHex(ContractJobManagerBin), backend, _relayer, _coprocessorOperator)
	if err != nil {
		return common.Address{}, nil, nil, err
	}
	return address, tx, &ContractJobManager{ContractJobManagerCaller: ContractJobManagerCaller{contract: contract}, ContractJobManagerTransactor: ContractJobManagerTransactor{contract: contract}, ContractJobManagerFilterer: ContractJobManagerFilterer{contract: contract}}, nil
}

// ContractJobManager is an auto generated Go binding around an Ethereum contract.
type ContractJobManager struct {
	ContractJobManagerCaller     // Read-only binding to the contract
	ContractJobManagerTransactor // Write-only binding to the contract
	ContractJobManagerFilterer   // Log filterer for contract events
}

// ContractJobManagerCaller is an auto generated read-only Go binding around an Ethereum contract.
type ContractJobManagerCaller struct {
	contract *bind.BoundContract // Generic contract wrapper for the low level calls
}

// ContractJobManagerTransactor is an auto generated write-only Go binding around an Ethereum contract.
type ContractJobManagerTransactor struct {
	contract *bind.BoundContract // Generic contract wrapper for the low level calls
}

// ContractJobManagerFilterer is an auto generated log filtering Go binding around an Ethereum contract events.
type ContractJobManagerFilterer struct {
	contract *bind.BoundContract // Generic contract wrapper for the low level calls
}

// ContractJobManagerSession is an auto generated Go binding around an Ethereum contract,
// with pre-set call and transact options.
type ContractJobManagerSession struct {
	Contract     *ContractJobManager // Generic contract binding to set the session for
	CallOpts     bind.CallOpts       // Call options to use throughout this session
	TransactOpts bind.TransactOpts   // Transaction auth options to use throughout this session
}

// ContractJobManagerCallerSession is an auto generated read-only Go binding around an Ethereum contract,
// with pre-set call options.
type ContractJobManagerCallerSession struct {
	Contract *ContractJobManagerCaller // Generic contract caller binding to set the session for
	CallOpts bind.CallOpts             // Call options to use throughout this session
}

// ContractJobManagerTransactorSession is an auto generated write-only Go binding around an Ethereum contract,
// with pre-set transact options.
type ContractJobManagerTransactorSession struct {
	Contract     *ContractJobManagerTransactor // Generic contract transactor binding to set the session for
	TransactOpts bind.TransactOpts             // Transaction auth options to use throughout this session
}

// ContractJobManagerRaw is an auto generated low-level Go binding around an Ethereum contract.
type ContractJobManagerRaw struct {
	Contract *ContractJobManager // Generic contract binding to access the raw methods on
}

// ContractJobManagerCallerRaw is an auto generated low-level read-only Go binding around an Ethereum contract.
type ContractJobManagerCallerRaw struct {
	Contract *ContractJobManagerCaller // Generic read-only contract binding to access the raw methods on
}

// ContractJobManagerTransactorRaw is an auto generated low-level write-only Go binding around an Ethereum contract.
type ContractJobManagerTransactorRaw struct {
	Contract *ContractJobManagerTransactor // Generic write-only contract binding to access the raw methods on
}

// NewContractJobManager creates a new instance of ContractJobManager, bound to a specific deployed contract.
func NewContractJobManager(address common.Address, backend bind.ContractBackend) (*ContractJobManager, error) {
	contract, err := bindContractJobManager(address, backend, backend, backend)
	if err != nil {
		return nil, err
	}
	return &ContractJobManager{ContractJobManagerCaller: ContractJobManagerCaller{contract: contract}, ContractJobManagerTransactor: ContractJobManagerTransactor{contract: contract}, ContractJobManagerFilterer: ContractJobManagerFilterer{contract: contract}}, nil
}

// NewContractJobManagerCaller creates a new read-only instance of ContractJobManager, bound to a specific deployed contract.
func NewContractJobManagerCaller(address common.Address, caller bind.ContractCaller) (*ContractJobManagerCaller, error) {
	contract, err := bindContractJobManager(address, caller, nil, nil)
	if err != nil {
		return nil, err
	}
	return &ContractJobManagerCaller{contract: contract}, nil
}

// NewContractJobManagerTransactor creates a new write-only instance of ContractJobManager, bound to a specific deployed contract.
func NewContractJobManagerTransactor(address common.Address, transactor bind.ContractTransactor) (*ContractJobManagerTransactor, error) {
	contract, err := bindContractJobManager(address, nil, transactor, nil)
	if err != nil {
		return nil, err
	}
	return &ContractJobManagerTransactor{contract: contract}, nil
}

// NewContractJobManagerFilterer creates a new log filterer instance of ContractJobManager, bound to a specific deployed contract.
func NewContractJobManagerFilterer(address common.Address, filterer bind.ContractFilterer) (*ContractJobManagerFilterer, error) {
	contract, err := bindContractJobManager(address, nil, nil, filterer)
	if err != nil {
		return nil, err
	}
	return &ContractJobManagerFilterer{contract: contract}, nil
}

// bindContractJobManager binds a generic wrapper to an already deployed contract.
func bindContractJobManager(address common.Address, caller bind.ContractCaller, transactor bind.ContractTransactor, filterer bind.ContractFilterer) (*bind.BoundContract, error) {
	parsed, err := ContractJobManagerMetaData.GetAbi()
	if err != nil {
		return nil, err
	}
	return bind.NewBoundContract(address, *parsed, caller, transactor, filterer), nil
}

// Call invokes the (constant) contract method with params as input values and
// sets the output to result. The result type might be a single field for simple
// returns, a slice of interfaces for anonymous returns and a struct for named
// returns.
func (_ContractJobManager *ContractJobManagerRaw) Call(opts *bind.CallOpts, result *[]interface{}, method string, params ...interface{}) error {
	return _ContractJobManager.Contract.ContractJobManagerCaller.contract.Call(opts, result, method, params...)
}

// Transfer initiates a plain transaction to move funds to the contract, calling
// its default method if one is available.
func (_ContractJobManager *ContractJobManagerRaw) Transfer(opts *bind.TransactOpts) (*types.Transaction, error) {
	return _ContractJobManager.Contract.ContractJobManagerTransactor.contract.Transfer(opts)
}

// Transact invokes the (paid) contract method with params as input values.
func (_ContractJobManager *ContractJobManagerRaw) Transact(opts *bind.TransactOpts, method string, params ...interface{}) (*types.Transaction, error) {
	return _ContractJobManager.Contract.ContractJobManagerTransactor.contract.Transact(opts, method, params...)
}

// Call invokes the (constant) contract method with params as input values and
// sets the output to result. The result type might be a single field for simple
// returns, a slice of interfaces for anonymous returns and a struct for named
// returns.
func (_ContractJobManager *ContractJobManagerCallerRaw) Call(opts *bind.CallOpts, result *[]interface{}, method string, params ...interface{}) error {
	return _ContractJobManager.Contract.contract.Call(opts, result, method, params...)
}

// Transfer initiates a plain transaction to move funds to the contract, calling
// its default method if one is available.
func (_ContractJobManager *ContractJobManagerTransactorRaw) Transfer(opts *bind.TransactOpts) (*types.Transaction, error) {
	return _ContractJobManager.Contract.contract.Transfer(opts)
}

// Transact invokes the (paid) contract method with params as input values.
func (_ContractJobManager *ContractJobManagerTransactorRaw) Transact(opts *bind.TransactOpts, method string, params ...interface{}) (*types.Transaction, error) {
	return _ContractJobManager.Contract.contract.Transact(opts, method, params...)
}

// GetJobMetadata is a free data retrieval call binding the contract method 0xf9918a80.
//
// Solidity: function getJobMetadata(uint32 jobID) view returns((bytes,address,uint8))
func (_ContractJobManager *ContractJobManagerCaller) GetJobMetadata(opts *bind.CallOpts, jobID uint32) (IJobManagerJobMetadata, error) {
	var out []interface{}
	err := _ContractJobManager.contract.Call(opts, &out, "getJobMetadata", jobID)

	if err != nil {
		return *new(IJobManagerJobMetadata), err
	}

	out0 := *abi.ConvertType(out[0], new(IJobManagerJobMetadata)).(*IJobManagerJobMetadata)

	return out0, err

}

// GetJobMetadata is a free data retrieval call binding the contract method 0xf9918a80.
//
// Solidity: function getJobMetadata(uint32 jobID) view returns((bytes,address,uint8))
func (_ContractJobManager *ContractJobManagerSession) GetJobMetadata(jobID uint32) (IJobManagerJobMetadata, error) {
	return _ContractJobManager.Contract.GetJobMetadata(&_ContractJobManager.CallOpts, jobID)
}

// GetJobMetadata is a free data retrieval call binding the contract method 0xf9918a80.
//
// Solidity: function getJobMetadata(uint32 jobID) view returns((bytes,address,uint8))
func (_ContractJobManager *ContractJobManagerCallerSession) GetJobMetadata(jobID uint32) (IJobManagerJobMetadata, error) {
	return _ContractJobManager.Contract.GetJobMetadata(&_ContractJobManager.CallOpts, jobID)
}

// JobIDCounter is a free data retrieval call binding the contract method 0xd7967c07.
//
// Solidity: function jobIDCounter() view returns(uint32)
func (_ContractJobManager *ContractJobManagerCaller) JobIDCounter(opts *bind.CallOpts) (uint32, error) {
	var out []interface{}
	err := _ContractJobManager.contract.Call(opts, &out, "jobIDCounter")

	if err != nil {
		return *new(uint32), err
	}

	out0 := *abi.ConvertType(out[0], new(uint32)).(*uint32)

	return out0, err

}

// JobIDCounter is a free data retrieval call binding the contract method 0xd7967c07.
//
// Solidity: function jobIDCounter() view returns(uint32)
func (_ContractJobManager *ContractJobManagerSession) JobIDCounter() (uint32, error) {
	return _ContractJobManager.Contract.JobIDCounter(&_ContractJobManager.CallOpts)
}

// JobIDCounter is a free data retrieval call binding the contract method 0xd7967c07.
//
// Solidity: function jobIDCounter() view returns(uint32)
func (_ContractJobManager *ContractJobManagerCallerSession) JobIDCounter() (uint32, error) {
	return _ContractJobManager.Contract.JobIDCounter(&_ContractJobManager.CallOpts)
}

// JobIDToMetadata is a free data retrieval call binding the contract method 0x71a7fd92.
//
// Solidity: function jobIDToMetadata(uint32 ) view returns(bytes programID, address caller, uint8 status)
func (_ContractJobManager *ContractJobManagerCaller) JobIDToMetadata(opts *bind.CallOpts, arg0 uint32) (struct {
	ProgramID []byte
	Caller    common.Address
	Status    uint8
}, error) {
	var out []interface{}
	err := _ContractJobManager.contract.Call(opts, &out, "jobIDToMetadata", arg0)

	outstruct := new(struct {
		ProgramID []byte
		Caller    common.Address
		Status    uint8
	})
	if err != nil {
		return *outstruct, err
	}

	outstruct.ProgramID = *abi.ConvertType(out[0], new([]byte)).(*[]byte)
	outstruct.Caller = *abi.ConvertType(out[1], new(common.Address)).(*common.Address)
	outstruct.Status = *abi.ConvertType(out[2], new(uint8)).(*uint8)

	return *outstruct, err

}

// JobIDToMetadata is a free data retrieval call binding the contract method 0x71a7fd92.
//
// Solidity: function jobIDToMetadata(uint32 ) view returns(bytes programID, address caller, uint8 status)
func (_ContractJobManager *ContractJobManagerSession) JobIDToMetadata(arg0 uint32) (struct {
	ProgramID []byte
	Caller    common.Address
	Status    uint8
}, error) {
	return _ContractJobManager.Contract.JobIDToMetadata(&_ContractJobManager.CallOpts, arg0)
}

// JobIDToMetadata is a free data retrieval call binding the contract method 0x71a7fd92.
//
// Solidity: function jobIDToMetadata(uint32 ) view returns(bytes programID, address caller, uint8 status)
func (_ContractJobManager *ContractJobManagerCallerSession) JobIDToMetadata(arg0 uint32) (struct {
	ProgramID []byte
	Caller    common.Address
	Status    uint8
}, error) {
	return _ContractJobManager.Contract.JobIDToMetadata(&_ContractJobManager.CallOpts, arg0)
}

// Owner is a free data retrieval call binding the contract method 0x8da5cb5b.
//
// Solidity: function owner() view returns(address)
func (_ContractJobManager *ContractJobManagerCaller) Owner(opts *bind.CallOpts) (common.Address, error) {
	var out []interface{}
	err := _ContractJobManager.contract.Call(opts, &out, "owner")

	if err != nil {
		return *new(common.Address), err
	}

	out0 := *abi.ConvertType(out[0], new(common.Address)).(*common.Address)

	return out0, err

}

// Owner is a free data retrieval call binding the contract method 0x8da5cb5b.
//
// Solidity: function owner() view returns(address)
func (_ContractJobManager *ContractJobManagerSession) Owner() (common.Address, error) {
	return _ContractJobManager.Contract.Owner(&_ContractJobManager.CallOpts)
}

// Owner is a free data retrieval call binding the contract method 0x8da5cb5b.
//
// Solidity: function owner() view returns(address)
func (_ContractJobManager *ContractJobManagerCallerSession) Owner() (common.Address, error) {
	return _ContractJobManager.Contract.Owner(&_ContractJobManager.CallOpts)
}

// CancelJob is a paid mutator transaction binding the contract method 0x8eb533ef.
//
// Solidity: function cancelJob(uint32 jobID) returns()
func (_ContractJobManager *ContractJobManagerTransactor) CancelJob(opts *bind.TransactOpts, jobID uint32) (*types.Transaction, error) {
	return _ContractJobManager.contract.Transact(opts, "cancelJob", jobID)
}

// CancelJob is a paid mutator transaction binding the contract method 0x8eb533ef.
//
// Solidity: function cancelJob(uint32 jobID) returns()
func (_ContractJobManager *ContractJobManagerSession) CancelJob(jobID uint32) (*types.Transaction, error) {
	return _ContractJobManager.Contract.CancelJob(&_ContractJobManager.TransactOpts, jobID)
}

// CancelJob is a paid mutator transaction binding the contract method 0x8eb533ef.
//
// Solidity: function cancelJob(uint32 jobID) returns()
func (_ContractJobManager *ContractJobManagerTransactorSession) CancelJob(jobID uint32) (*types.Transaction, error) {
	return _ContractJobManager.Contract.CancelJob(&_ContractJobManager.TransactOpts, jobID)
}

// CreateJob is a paid mutator transaction binding the contract method 0xb577a6e4.
//
// Solidity: function createJob(bytes programID, bytes programInput) returns(uint32 jobID)
func (_ContractJobManager *ContractJobManagerTransactor) CreateJob(opts *bind.TransactOpts, programID []byte, programInput []byte) (*types.Transaction, error) {
	return _ContractJobManager.contract.Transact(opts, "createJob", programID, programInput)
}

// CreateJob is a paid mutator transaction binding the contract method 0xb577a6e4.
//
// Solidity: function createJob(bytes programID, bytes programInput) returns(uint32 jobID)
func (_ContractJobManager *ContractJobManagerSession) CreateJob(programID []byte, programInput []byte) (*types.Transaction, error) {
	return _ContractJobManager.Contract.CreateJob(&_ContractJobManager.TransactOpts, programID, programInput)
}

// CreateJob is a paid mutator transaction binding the contract method 0xb577a6e4.
//
// Solidity: function createJob(bytes programID, bytes programInput) returns(uint32 jobID)
func (_ContractJobManager *ContractJobManagerTransactorSession) CreateJob(programID []byte, programInput []byte) (*types.Transaction, error) {
	return _ContractJobManager.Contract.CreateJob(&_ContractJobManager.TransactOpts, programID, programInput)
}

// InitializeJobManager is a paid mutator transaction binding the contract method 0x62020438.
//
// Solidity: function initializeJobManager(address initialOwner) returns()
func (_ContractJobManager *ContractJobManagerTransactor) InitializeJobManager(opts *bind.TransactOpts, initialOwner common.Address) (*types.Transaction, error) {
	return _ContractJobManager.contract.Transact(opts, "initializeJobManager", initialOwner)
}

// InitializeJobManager is a paid mutator transaction binding the contract method 0x62020438.
//
// Solidity: function initializeJobManager(address initialOwner) returns()
func (_ContractJobManager *ContractJobManagerSession) InitializeJobManager(initialOwner common.Address) (*types.Transaction, error) {
	return _ContractJobManager.Contract.InitializeJobManager(&_ContractJobManager.TransactOpts, initialOwner)
}

// InitializeJobManager is a paid mutator transaction binding the contract method 0x62020438.
//
// Solidity: function initializeJobManager(address initialOwner) returns()
func (_ContractJobManager *ContractJobManagerTransactorSession) InitializeJobManager(initialOwner common.Address) (*types.Transaction, error) {
	return _ContractJobManager.Contract.InitializeJobManager(&_ContractJobManager.TransactOpts, initialOwner)
}

// RenounceOwnership is a paid mutator transaction binding the contract method 0x715018a6.
//
// Solidity: function renounceOwnership() returns()
func (_ContractJobManager *ContractJobManagerTransactor) RenounceOwnership(opts *bind.TransactOpts) (*types.Transaction, error) {
	return _ContractJobManager.contract.Transact(opts, "renounceOwnership")
}

// RenounceOwnership is a paid mutator transaction binding the contract method 0x715018a6.
//
// Solidity: function renounceOwnership() returns()
func (_ContractJobManager *ContractJobManagerSession) RenounceOwnership() (*types.Transaction, error) {
	return _ContractJobManager.Contract.RenounceOwnership(&_ContractJobManager.TransactOpts)
}

// RenounceOwnership is a paid mutator transaction binding the contract method 0x715018a6.
//
// Solidity: function renounceOwnership() returns()
func (_ContractJobManager *ContractJobManagerTransactorSession) RenounceOwnership() (*types.Transaction, error) {
	return _ContractJobManager.Contract.RenounceOwnership(&_ContractJobManager.TransactOpts)
}

// SetCoprocessorOperator is a paid mutator transaction binding the contract method 0x33db8f79.
//
// Solidity: function setCoprocessorOperator(address _coprocessorOperator) returns()
func (_ContractJobManager *ContractJobManagerTransactor) SetCoprocessorOperator(opts *bind.TransactOpts, _coprocessorOperator common.Address) (*types.Transaction, error) {
	return _ContractJobManager.contract.Transact(opts, "setCoprocessorOperator", _coprocessorOperator)
}

// SetCoprocessorOperator is a paid mutator transaction binding the contract method 0x33db8f79.
//
// Solidity: function setCoprocessorOperator(address _coprocessorOperator) returns()
func (_ContractJobManager *ContractJobManagerSession) SetCoprocessorOperator(_coprocessorOperator common.Address) (*types.Transaction, error) {
	return _ContractJobManager.Contract.SetCoprocessorOperator(&_ContractJobManager.TransactOpts, _coprocessorOperator)
}

// SetCoprocessorOperator is a paid mutator transaction binding the contract method 0x33db8f79.
//
// Solidity: function setCoprocessorOperator(address _coprocessorOperator) returns()
func (_ContractJobManager *ContractJobManagerTransactorSession) SetCoprocessorOperator(_coprocessorOperator common.Address) (*types.Transaction, error) {
	return _ContractJobManager.Contract.SetCoprocessorOperator(&_ContractJobManager.TransactOpts, _coprocessorOperator)
}

// SetRelayer is a paid mutator transaction binding the contract method 0x6548e9bc.
//
// Solidity: function setRelayer(address _relayer) returns()
func (_ContractJobManager *ContractJobManagerTransactor) SetRelayer(opts *bind.TransactOpts, _relayer common.Address) (*types.Transaction, error) {
	return _ContractJobManager.contract.Transact(opts, "setRelayer", _relayer)
}

// SetRelayer is a paid mutator transaction binding the contract method 0x6548e9bc.
//
// Solidity: function setRelayer(address _relayer) returns()
func (_ContractJobManager *ContractJobManagerSession) SetRelayer(_relayer common.Address) (*types.Transaction, error) {
	return _ContractJobManager.Contract.SetRelayer(&_ContractJobManager.TransactOpts, _relayer)
}

// SetRelayer is a paid mutator transaction binding the contract method 0x6548e9bc.
//
// Solidity: function setRelayer(address _relayer) returns()
func (_ContractJobManager *ContractJobManagerTransactorSession) SetRelayer(_relayer common.Address) (*types.Transaction, error) {
	return _ContractJobManager.Contract.SetRelayer(&_ContractJobManager.TransactOpts, _relayer)
}

// SubmitResult is a paid mutator transaction binding the contract method 0xea253487.
//
// Solidity: function submitResult(bytes resultWithMetadata, bytes signature) returns()
func (_ContractJobManager *ContractJobManagerTransactor) SubmitResult(opts *bind.TransactOpts, resultWithMetadata []byte, signature []byte) (*types.Transaction, error) {
	return _ContractJobManager.contract.Transact(opts, "submitResult", resultWithMetadata, signature)
}

// SubmitResult is a paid mutator transaction binding the contract method 0xea253487.
//
// Solidity: function submitResult(bytes resultWithMetadata, bytes signature) returns()
func (_ContractJobManager *ContractJobManagerSession) SubmitResult(resultWithMetadata []byte, signature []byte) (*types.Transaction, error) {
	return _ContractJobManager.Contract.SubmitResult(&_ContractJobManager.TransactOpts, resultWithMetadata, signature)
}

// SubmitResult is a paid mutator transaction binding the contract method 0xea253487.
//
// Solidity: function submitResult(bytes resultWithMetadata, bytes signature) returns()
func (_ContractJobManager *ContractJobManagerTransactorSession) SubmitResult(resultWithMetadata []byte, signature []byte) (*types.Transaction, error) {
	return _ContractJobManager.Contract.SubmitResult(&_ContractJobManager.TransactOpts, resultWithMetadata, signature)
}

// TransferOwnership is a paid mutator transaction binding the contract method 0xf2fde38b.
//
// Solidity: function transferOwnership(address newOwner) returns()
func (_ContractJobManager *ContractJobManagerTransactor) TransferOwnership(opts *bind.TransactOpts, newOwner common.Address) (*types.Transaction, error) {
	return _ContractJobManager.contract.Transact(opts, "transferOwnership", newOwner)
}

// TransferOwnership is a paid mutator transaction binding the contract method 0xf2fde38b.
//
// Solidity: function transferOwnership(address newOwner) returns()
func (_ContractJobManager *ContractJobManagerSession) TransferOwnership(newOwner common.Address) (*types.Transaction, error) {
	return _ContractJobManager.Contract.TransferOwnership(&_ContractJobManager.TransactOpts, newOwner)
}

// TransferOwnership is a paid mutator transaction binding the contract method 0xf2fde38b.
//
// Solidity: function transferOwnership(address newOwner) returns()
func (_ContractJobManager *ContractJobManagerTransactorSession) TransferOwnership(newOwner common.Address) (*types.Transaction, error) {
	return _ContractJobManager.Contract.TransferOwnership(&_ContractJobManager.TransactOpts, newOwner)
}

// ContractJobManagerInitializedIterator is returned from FilterInitialized and is used to iterate over the raw logs and unpacked data for Initialized events raised by the ContractJobManager contract.
type ContractJobManagerInitializedIterator struct {
	Event *ContractJobManagerInitialized // Event containing the contract specifics and raw log

	contract *bind.BoundContract // Generic contract to use for unpacking event data
	event    string              // Event name to use for unpacking event data

	logs chan types.Log        // Log channel receiving the found contract events
	sub  ethereum.Subscription // Subscription for errors, completion and termination
	done bool                  // Whether the subscription completed delivering logs
	fail error                 // Occurred error to stop iteration
}

// Next advances the iterator to the subsequent event, returning whether there
// are any more events found. In case of a retrieval or parsing error, false is
// returned and Error() can be queried for the exact failure.
func (it *ContractJobManagerInitializedIterator) Next() bool {
	// If the iterator failed, stop iterating
	if it.fail != nil {
		return false
	}
	// If the iterator completed, deliver directly whatever's available
	if it.done {
		select {
		case log := <-it.logs:
			it.Event = new(ContractJobManagerInitialized)
			if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
				it.fail = err
				return false
			}
			it.Event.Raw = log
			return true

		default:
			return false
		}
	}
	// Iterator still in progress, wait for either a data or an error event
	select {
	case log := <-it.logs:
		it.Event = new(ContractJobManagerInitialized)
		if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
			it.fail = err
			return false
		}
		it.Event.Raw = log
		return true

	case err := <-it.sub.Err():
		it.done = true
		it.fail = err
		return it.Next()
	}
}

// Error returns any retrieval or parsing error occurred during filtering.
func (it *ContractJobManagerInitializedIterator) Error() error {
	return it.fail
}

// Close terminates the iteration process, releasing any pending underlying
// resources.
func (it *ContractJobManagerInitializedIterator) Close() error {
	it.sub.Unsubscribe()
	return nil
}

// ContractJobManagerInitialized represents a Initialized event raised by the ContractJobManager contract.
type ContractJobManagerInitialized struct {
	Version uint64
	Raw     types.Log // Blockchain specific contextual infos
}

// FilterInitialized is a free log retrieval operation binding the contract event 0xc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d2.
//
// Solidity: event Initialized(uint64 version)
func (_ContractJobManager *ContractJobManagerFilterer) FilterInitialized(opts *bind.FilterOpts) (*ContractJobManagerInitializedIterator, error) {

	logs, sub, err := _ContractJobManager.contract.FilterLogs(opts, "Initialized")
	if err != nil {
		return nil, err
	}
	return &ContractJobManagerInitializedIterator{contract: _ContractJobManager.contract, event: "Initialized", logs: logs, sub: sub}, nil
}

// WatchInitialized is a free log subscription operation binding the contract event 0xc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d2.
//
// Solidity: event Initialized(uint64 version)
func (_ContractJobManager *ContractJobManagerFilterer) WatchInitialized(opts *bind.WatchOpts, sink chan<- *ContractJobManagerInitialized) (event.Subscription, error) {

	logs, sub, err := _ContractJobManager.contract.WatchLogs(opts, "Initialized")
	if err != nil {
		return nil, err
	}
	return event.NewSubscription(func(quit <-chan struct{}) error {
		defer sub.Unsubscribe()
		for {
			select {
			case log := <-logs:
				// New log arrived, parse the event and forward to the user
				event := new(ContractJobManagerInitialized)
				if err := _ContractJobManager.contract.UnpackLog(event, "Initialized", log); err != nil {
					return err
				}
				event.Raw = log

				select {
				case sink <- event:
				case err := <-sub.Err():
					return err
				case <-quit:
					return nil
				}
			case err := <-sub.Err():
				return err
			case <-quit:
				return nil
			}
		}
	}), nil
}

// ParseInitialized is a log parse operation binding the contract event 0xc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d2.
//
// Solidity: event Initialized(uint64 version)
func (_ContractJobManager *ContractJobManagerFilterer) ParseInitialized(log types.Log) (*ContractJobManagerInitialized, error) {
	event := new(ContractJobManagerInitialized)
	if err := _ContractJobManager.contract.UnpackLog(event, "Initialized", log); err != nil {
		return nil, err
	}
	event.Raw = log
	return event, nil
}

// ContractJobManagerJobCancelledIterator is returned from FilterJobCancelled and is used to iterate over the raw logs and unpacked data for JobCancelled events raised by the ContractJobManager contract.
type ContractJobManagerJobCancelledIterator struct {
	Event *ContractJobManagerJobCancelled // Event containing the contract specifics and raw log

	contract *bind.BoundContract // Generic contract to use for unpacking event data
	event    string              // Event name to use for unpacking event data

	logs chan types.Log        // Log channel receiving the found contract events
	sub  ethereum.Subscription // Subscription for errors, completion and termination
	done bool                  // Whether the subscription completed delivering logs
	fail error                 // Occurred error to stop iteration
}

// Next advances the iterator to the subsequent event, returning whether there
// are any more events found. In case of a retrieval or parsing error, false is
// returned and Error() can be queried for the exact failure.
func (it *ContractJobManagerJobCancelledIterator) Next() bool {
	// If the iterator failed, stop iterating
	if it.fail != nil {
		return false
	}
	// If the iterator completed, deliver directly whatever's available
	if it.done {
		select {
		case log := <-it.logs:
			it.Event = new(ContractJobManagerJobCancelled)
			if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
				it.fail = err
				return false
			}
			it.Event.Raw = log
			return true

		default:
			return false
		}
	}
	// Iterator still in progress, wait for either a data or an error event
	select {
	case log := <-it.logs:
		it.Event = new(ContractJobManagerJobCancelled)
		if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
			it.fail = err
			return false
		}
		it.Event.Raw = log
		return true

	case err := <-it.sub.Err():
		it.done = true
		it.fail = err
		return it.Next()
	}
}

// Error returns any retrieval or parsing error occurred during filtering.
func (it *ContractJobManagerJobCancelledIterator) Error() error {
	return it.fail
}

// Close terminates the iteration process, releasing any pending underlying
// resources.
func (it *ContractJobManagerJobCancelledIterator) Close() error {
	it.sub.Unsubscribe()
	return nil
}

// ContractJobManagerJobCancelled represents a JobCancelled event raised by the ContractJobManager contract.
type ContractJobManagerJobCancelled struct {
	JobID uint32
	Raw   types.Log // Blockchain specific contextual infos
}

// FilterJobCancelled is a free log retrieval operation binding the contract event 0xa6a2eb5c45aee3821a738c1beb0dade684d1189539bab9d4f634c00de2449e7a.
//
// Solidity: event JobCancelled(uint32 indexed jobID)
func (_ContractJobManager *ContractJobManagerFilterer) FilterJobCancelled(opts *bind.FilterOpts, jobID []uint32) (*ContractJobManagerJobCancelledIterator, error) {

	var jobIDRule []interface{}
	for _, jobIDItem := range jobID {
		jobIDRule = append(jobIDRule, jobIDItem)
	}

	logs, sub, err := _ContractJobManager.contract.FilterLogs(opts, "JobCancelled", jobIDRule)
	if err != nil {
		return nil, err
	}
	return &ContractJobManagerJobCancelledIterator{contract: _ContractJobManager.contract, event: "JobCancelled", logs: logs, sub: sub}, nil
}

// WatchJobCancelled is a free log subscription operation binding the contract event 0xa6a2eb5c45aee3821a738c1beb0dade684d1189539bab9d4f634c00de2449e7a.
//
// Solidity: event JobCancelled(uint32 indexed jobID)
func (_ContractJobManager *ContractJobManagerFilterer) WatchJobCancelled(opts *bind.WatchOpts, sink chan<- *ContractJobManagerJobCancelled, jobID []uint32) (event.Subscription, error) {

	var jobIDRule []interface{}
	for _, jobIDItem := range jobID {
		jobIDRule = append(jobIDRule, jobIDItem)
	}

	logs, sub, err := _ContractJobManager.contract.WatchLogs(opts, "JobCancelled", jobIDRule)
	if err != nil {
		return nil, err
	}
	return event.NewSubscription(func(quit <-chan struct{}) error {
		defer sub.Unsubscribe()
		for {
			select {
			case log := <-logs:
				// New log arrived, parse the event and forward to the user
				event := new(ContractJobManagerJobCancelled)
				if err := _ContractJobManager.contract.UnpackLog(event, "JobCancelled", log); err != nil {
					return err
				}
				event.Raw = log

				select {
				case sink <- event:
				case err := <-sub.Err():
					return err
				case <-quit:
					return nil
				}
			case err := <-sub.Err():
				return err
			case <-quit:
				return nil
			}
		}
	}), nil
}

// ParseJobCancelled is a log parse operation binding the contract event 0xa6a2eb5c45aee3821a738c1beb0dade684d1189539bab9d4f634c00de2449e7a.
//
// Solidity: event JobCancelled(uint32 indexed jobID)
func (_ContractJobManager *ContractJobManagerFilterer) ParseJobCancelled(log types.Log) (*ContractJobManagerJobCancelled, error) {
	event := new(ContractJobManagerJobCancelled)
	if err := _ContractJobManager.contract.UnpackLog(event, "JobCancelled", log); err != nil {
		return nil, err
	}
	event.Raw = log
	return event, nil
}

// ContractJobManagerJobCompletedIterator is returned from FilterJobCompleted and is used to iterate over the raw logs and unpacked data for JobCompleted events raised by the ContractJobManager contract.
type ContractJobManagerJobCompletedIterator struct {
	Event *ContractJobManagerJobCompleted // Event containing the contract specifics and raw log

	contract *bind.BoundContract // Generic contract to use for unpacking event data
	event    string              // Event name to use for unpacking event data

	logs chan types.Log        // Log channel receiving the found contract events
	sub  ethereum.Subscription // Subscription for errors, completion and termination
	done bool                  // Whether the subscription completed delivering logs
	fail error                 // Occurred error to stop iteration
}

// Next advances the iterator to the subsequent event, returning whether there
// are any more events found. In case of a retrieval or parsing error, false is
// returned and Error() can be queried for the exact failure.
func (it *ContractJobManagerJobCompletedIterator) Next() bool {
	// If the iterator failed, stop iterating
	if it.fail != nil {
		return false
	}
	// If the iterator completed, deliver directly whatever's available
	if it.done {
		select {
		case log := <-it.logs:
			it.Event = new(ContractJobManagerJobCompleted)
			if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
				it.fail = err
				return false
			}
			it.Event.Raw = log
			return true

		default:
			return false
		}
	}
	// Iterator still in progress, wait for either a data or an error event
	select {
	case log := <-it.logs:
		it.Event = new(ContractJobManagerJobCompleted)
		if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
			it.fail = err
			return false
		}
		it.Event.Raw = log
		return true

	case err := <-it.sub.Err():
		it.done = true
		it.fail = err
		return it.Next()
	}
}

// Error returns any retrieval or parsing error occurred during filtering.
func (it *ContractJobManagerJobCompletedIterator) Error() error {
	return it.fail
}

// Close terminates the iteration process, releasing any pending underlying
// resources.
func (it *ContractJobManagerJobCompletedIterator) Close() error {
	it.sub.Unsubscribe()
	return nil
}

// ContractJobManagerJobCompleted represents a JobCompleted event raised by the ContractJobManager contract.
type ContractJobManagerJobCompleted struct {
	JobID  uint32
	Result []byte
	Raw    types.Log // Blockchain specific contextual infos
}

// FilterJobCompleted is a free log retrieval operation binding the contract event 0xd75a350cc0ba7be9680891736117e26b5fdf4bff5685a8a9346af44d87022be5.
//
// Solidity: event JobCompleted(uint32 indexed jobID, bytes result)
func (_ContractJobManager *ContractJobManagerFilterer) FilterJobCompleted(opts *bind.FilterOpts, jobID []uint32) (*ContractJobManagerJobCompletedIterator, error) {

	var jobIDRule []interface{}
	for _, jobIDItem := range jobID {
		jobIDRule = append(jobIDRule, jobIDItem)
	}

	logs, sub, err := _ContractJobManager.contract.FilterLogs(opts, "JobCompleted", jobIDRule)
	if err != nil {
		return nil, err
	}
	return &ContractJobManagerJobCompletedIterator{contract: _ContractJobManager.contract, event: "JobCompleted", logs: logs, sub: sub}, nil
}

// WatchJobCompleted is a free log subscription operation binding the contract event 0xd75a350cc0ba7be9680891736117e26b5fdf4bff5685a8a9346af44d87022be5.
//
// Solidity: event JobCompleted(uint32 indexed jobID, bytes result)
func (_ContractJobManager *ContractJobManagerFilterer) WatchJobCompleted(opts *bind.WatchOpts, sink chan<- *ContractJobManagerJobCompleted, jobID []uint32) (event.Subscription, error) {

	var jobIDRule []interface{}
	for _, jobIDItem := range jobID {
		jobIDRule = append(jobIDRule, jobIDItem)
	}

	logs, sub, err := _ContractJobManager.contract.WatchLogs(opts, "JobCompleted", jobIDRule)
	if err != nil {
		return nil, err
	}
	return event.NewSubscription(func(quit <-chan struct{}) error {
		defer sub.Unsubscribe()
		for {
			select {
			case log := <-logs:
				// New log arrived, parse the event and forward to the user
				event := new(ContractJobManagerJobCompleted)
				if err := _ContractJobManager.contract.UnpackLog(event, "JobCompleted", log); err != nil {
					return err
				}
				event.Raw = log

				select {
				case sink <- event:
				case err := <-sub.Err():
					return err
				case <-quit:
					return nil
				}
			case err := <-sub.Err():
				return err
			case <-quit:
				return nil
			}
		}
	}), nil
}

// ParseJobCompleted is a log parse operation binding the contract event 0xd75a350cc0ba7be9680891736117e26b5fdf4bff5685a8a9346af44d87022be5.
//
// Solidity: event JobCompleted(uint32 indexed jobID, bytes result)
func (_ContractJobManager *ContractJobManagerFilterer) ParseJobCompleted(log types.Log) (*ContractJobManagerJobCompleted, error) {
	event := new(ContractJobManagerJobCompleted)
	if err := _ContractJobManager.contract.UnpackLog(event, "JobCompleted", log); err != nil {
		return nil, err
	}
	event.Raw = log
	return event, nil
}

// ContractJobManagerJobCreatedIterator is returned from FilterJobCreated and is used to iterate over the raw logs and unpacked data for JobCreated events raised by the ContractJobManager contract.
type ContractJobManagerJobCreatedIterator struct {
	Event *ContractJobManagerJobCreated // Event containing the contract specifics and raw log

	contract *bind.BoundContract // Generic contract to use for unpacking event data
	event    string              // Event name to use for unpacking event data

	logs chan types.Log        // Log channel receiving the found contract events
	sub  ethereum.Subscription // Subscription for errors, completion and termination
	done bool                  // Whether the subscription completed delivering logs
	fail error                 // Occurred error to stop iteration
}

// Next advances the iterator to the subsequent event, returning whether there
// are any more events found. In case of a retrieval or parsing error, false is
// returned and Error() can be queried for the exact failure.
func (it *ContractJobManagerJobCreatedIterator) Next() bool {
	// If the iterator failed, stop iterating
	if it.fail != nil {
		return false
	}
	// If the iterator completed, deliver directly whatever's available
	if it.done {
		select {
		case log := <-it.logs:
			it.Event = new(ContractJobManagerJobCreated)
			if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
				it.fail = err
				return false
			}
			it.Event.Raw = log
			return true

		default:
			return false
		}
	}
	// Iterator still in progress, wait for either a data or an error event
	select {
	case log := <-it.logs:
		it.Event = new(ContractJobManagerJobCreated)
		if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
			it.fail = err
			return false
		}
		it.Event.Raw = log
		return true

	case err := <-it.sub.Err():
		it.done = true
		it.fail = err
		return it.Next()
	}
}

// Error returns any retrieval or parsing error occurred during filtering.
func (it *ContractJobManagerJobCreatedIterator) Error() error {
	return it.fail
}

// Close terminates the iteration process, releasing any pending underlying
// resources.
func (it *ContractJobManagerJobCreatedIterator) Close() error {
	it.sub.Unsubscribe()
	return nil
}

// ContractJobManagerJobCreated represents a JobCreated event raised by the ContractJobManager contract.
type ContractJobManagerJobCreated struct {
	JobID        uint32
	ProgramID    []byte
	ProgramInput []byte
	Raw          types.Log // Blockchain specific contextual infos
}

// FilterJobCreated is a free log retrieval operation binding the contract event 0x889a0aa153aefbb0af2faa68e340f3dbeb9ff2b45c378ae4c707bbaf13e3a924.
//
// Solidity: event JobCreated(uint32 indexed jobID, bytes programID, bytes programInput)
func (_ContractJobManager *ContractJobManagerFilterer) FilterJobCreated(opts *bind.FilterOpts, jobID []uint32) (*ContractJobManagerJobCreatedIterator, error) {

	var jobIDRule []interface{}
	for _, jobIDItem := range jobID {
		jobIDRule = append(jobIDRule, jobIDItem)
	}

	logs, sub, err := _ContractJobManager.contract.FilterLogs(opts, "JobCreated", jobIDRule)
	if err != nil {
		return nil, err
	}
	return &ContractJobManagerJobCreatedIterator{contract: _ContractJobManager.contract, event: "JobCreated", logs: logs, sub: sub}, nil
}

// WatchJobCreated is a free log subscription operation binding the contract event 0x889a0aa153aefbb0af2faa68e340f3dbeb9ff2b45c378ae4c707bbaf13e3a924.
//
// Solidity: event JobCreated(uint32 indexed jobID, bytes programID, bytes programInput)
func (_ContractJobManager *ContractJobManagerFilterer) WatchJobCreated(opts *bind.WatchOpts, sink chan<- *ContractJobManagerJobCreated, jobID []uint32) (event.Subscription, error) {

	var jobIDRule []interface{}
	for _, jobIDItem := range jobID {
		jobIDRule = append(jobIDRule, jobIDItem)
	}

	logs, sub, err := _ContractJobManager.contract.WatchLogs(opts, "JobCreated", jobIDRule)
	if err != nil {
		return nil, err
	}
	return event.NewSubscription(func(quit <-chan struct{}) error {
		defer sub.Unsubscribe()
		for {
			select {
			case log := <-logs:
				// New log arrived, parse the event and forward to the user
				event := new(ContractJobManagerJobCreated)
				if err := _ContractJobManager.contract.UnpackLog(event, "JobCreated", log); err != nil {
					return err
				}
				event.Raw = log

				select {
				case sink <- event:
				case err := <-sub.Err():
					return err
				case <-quit:
					return nil
				}
			case err := <-sub.Err():
				return err
			case <-quit:
				return nil
			}
		}
	}), nil
}

// ParseJobCreated is a log parse operation binding the contract event 0x889a0aa153aefbb0af2faa68e340f3dbeb9ff2b45c378ae4c707bbaf13e3a924.
//
// Solidity: event JobCreated(uint32 indexed jobID, bytes programID, bytes programInput)
func (_ContractJobManager *ContractJobManagerFilterer) ParseJobCreated(log types.Log) (*ContractJobManagerJobCreated, error) {
	event := new(ContractJobManagerJobCreated)
	if err := _ContractJobManager.contract.UnpackLog(event, "JobCreated", log); err != nil {
		return nil, err
	}
	event.Raw = log
	return event, nil
}

// ContractJobManagerOwnershipTransferredIterator is returned from FilterOwnershipTransferred and is used to iterate over the raw logs and unpacked data for OwnershipTransferred events raised by the ContractJobManager contract.
type ContractJobManagerOwnershipTransferredIterator struct {
	Event *ContractJobManagerOwnershipTransferred // Event containing the contract specifics and raw log

	contract *bind.BoundContract // Generic contract to use for unpacking event data
	event    string              // Event name to use for unpacking event data

	logs chan types.Log        // Log channel receiving the found contract events
	sub  ethereum.Subscription // Subscription for errors, completion and termination
	done bool                  // Whether the subscription completed delivering logs
	fail error                 // Occurred error to stop iteration
}

// Next advances the iterator to the subsequent event, returning whether there
// are any more events found. In case of a retrieval or parsing error, false is
// returned and Error() can be queried for the exact failure.
func (it *ContractJobManagerOwnershipTransferredIterator) Next() bool {
	// If the iterator failed, stop iterating
	if it.fail != nil {
		return false
	}
	// If the iterator completed, deliver directly whatever's available
	if it.done {
		select {
		case log := <-it.logs:
			it.Event = new(ContractJobManagerOwnershipTransferred)
			if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
				it.fail = err
				return false
			}
			it.Event.Raw = log
			return true

		default:
			return false
		}
	}
	// Iterator still in progress, wait for either a data or an error event
	select {
	case log := <-it.logs:
		it.Event = new(ContractJobManagerOwnershipTransferred)
		if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
			it.fail = err
			return false
		}
		it.Event.Raw = log
		return true

	case err := <-it.sub.Err():
		it.done = true
		it.fail = err
		return it.Next()
	}
}

// Error returns any retrieval or parsing error occurred during filtering.
func (it *ContractJobManagerOwnershipTransferredIterator) Error() error {
	return it.fail
}

// Close terminates the iteration process, releasing any pending underlying
// resources.
func (it *ContractJobManagerOwnershipTransferredIterator) Close() error {
	it.sub.Unsubscribe()
	return nil
}

// ContractJobManagerOwnershipTransferred represents a OwnershipTransferred event raised by the ContractJobManager contract.
type ContractJobManagerOwnershipTransferred struct {
	PreviousOwner common.Address
	NewOwner      common.Address
	Raw           types.Log // Blockchain specific contextual infos
}

// FilterOwnershipTransferred is a free log retrieval operation binding the contract event 0x8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0.
//
// Solidity: event OwnershipTransferred(address indexed previousOwner, address indexed newOwner)
func (_ContractJobManager *ContractJobManagerFilterer) FilterOwnershipTransferred(opts *bind.FilterOpts, previousOwner []common.Address, newOwner []common.Address) (*ContractJobManagerOwnershipTransferredIterator, error) {

	var previousOwnerRule []interface{}
	for _, previousOwnerItem := range previousOwner {
		previousOwnerRule = append(previousOwnerRule, previousOwnerItem)
	}
	var newOwnerRule []interface{}
	for _, newOwnerItem := range newOwner {
		newOwnerRule = append(newOwnerRule, newOwnerItem)
	}

	logs, sub, err := _ContractJobManager.contract.FilterLogs(opts, "OwnershipTransferred", previousOwnerRule, newOwnerRule)
	if err != nil {
		return nil, err
	}
	return &ContractJobManagerOwnershipTransferredIterator{contract: _ContractJobManager.contract, event: "OwnershipTransferred", logs: logs, sub: sub}, nil
}

// WatchOwnershipTransferred is a free log subscription operation binding the contract event 0x8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0.
//
// Solidity: event OwnershipTransferred(address indexed previousOwner, address indexed newOwner)
func (_ContractJobManager *ContractJobManagerFilterer) WatchOwnershipTransferred(opts *bind.WatchOpts, sink chan<- *ContractJobManagerOwnershipTransferred, previousOwner []common.Address, newOwner []common.Address) (event.Subscription, error) {

	var previousOwnerRule []interface{}
	for _, previousOwnerItem := range previousOwner {
		previousOwnerRule = append(previousOwnerRule, previousOwnerItem)
	}
	var newOwnerRule []interface{}
	for _, newOwnerItem := range newOwner {
		newOwnerRule = append(newOwnerRule, newOwnerItem)
	}

	logs, sub, err := _ContractJobManager.contract.WatchLogs(opts, "OwnershipTransferred", previousOwnerRule, newOwnerRule)
	if err != nil {
		return nil, err
	}
	return event.NewSubscription(func(quit <-chan struct{}) error {
		defer sub.Unsubscribe()
		for {
			select {
			case log := <-logs:
				// New log arrived, parse the event and forward to the user
				event := new(ContractJobManagerOwnershipTransferred)
				if err := _ContractJobManager.contract.UnpackLog(event, "OwnershipTransferred", log); err != nil {
					return err
				}
				event.Raw = log

				select {
				case sink <- event:
				case err := <-sub.Err():
					return err
				case <-quit:
					return nil
				}
			case err := <-sub.Err():
				return err
			case <-quit:
				return nil
			}
		}
	}), nil
}

// ParseOwnershipTransferred is a log parse operation binding the contract event 0x8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0.
//
// Solidity: event OwnershipTransferred(address indexed previousOwner, address indexed newOwner)
func (_ContractJobManager *ContractJobManagerFilterer) ParseOwnershipTransferred(log types.Log) (*ContractJobManagerOwnershipTransferred, error) {
	event := new(ContractJobManagerOwnershipTransferred)
	if err := _ContractJobManager.contract.UnpackLog(event, "OwnershipTransferred", log); err != nil {
		return nil, err
	}
	event.Raw = log
	return event, nil
}
