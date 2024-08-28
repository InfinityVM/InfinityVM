// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {JobManager} from "./JobManager.sol";
import {Consumer} from "./Consumer.sol";
import {OffchainRequester} from "./OffchainRequester.sol";
import {console} from "forge-std/Script.sol";
import {ECDSA} from "solady/utils/ECDSA.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract ClobConsumer is Consumer, OffchainRequester {
    // DepositDelta always subtracts from deposit balance, so we use uint256
    struct DepositDelta {
        address user;
        uint256 baseDelta;
        uint256 quoteDelta;
    }

    struct OrderDelta {
        address user;
        int256 freeBaseDelta;
        int256 lockedBaseDelta;
        int256 freeQuoteDelta;
        int256 lockedQuoteDelta;
    }

    // WithdrawDelta always subtracts from free balance, so we use uint256
    struct WithdrawDelta {
        address user;
        uint256 baseDelta;
        uint256 quoteDelta;
    }

    // Struct passed into zkVM program as input
    struct ClobProgramInput {
        bytes32 previousStateHash;
        // Borsh-encoded orders
        bytes orders;
    }

    // Struct returned by zkVM program as the result
    struct ClobProgramOutput {
        bytes32 nextStateRootHash;
        DepositDelta[] depositDeltas;
        OrderDelta[] orderDeltas;
        WithdrawDelta[] withdrawDeltas;
    }

    event Deposit(address indexed user, uint256 baseAmount, uint256 quoteAmount);

    address private offchainSigner;
    uint64 public constant DEFAULT_MAX_CYCLES = 32 * 1000 * 1000;

    // Define the two tokens
    IERC20 public baseToken;
    IERC20 public quoteToken;

    // Mappings to store balances for each user
    mapping(address => uint256) public depositedBalanceBase;
    mapping(address => uint256) public depositedBalanceQuote;
    mapping(address => uint256) public freeBalanceBase;
    mapping(address => uint256) public freeBalanceQuote;
    mapping(address => uint256) public lockedBalanceBase;
    mapping(address => uint256) public lockedBalanceQuote;

    bytes32 public latestStateRootHash;  

    constructor(address jobManager, address _offchainSigner, uint64 initialMaxNonce, IERC20 _baseToken, IERC20 _quoteToken) Consumer(jobManager, initialMaxNonce) OffchainRequester() {
        // ClobConsumer allows a single offchainSigner address to sign all offchain job requests
        offchainSigner = _offchainSigner;

        baseToken = _baseToken;
        quoteToken = _quoteToken;
    }

    // Getter functions for balances
    function getDepositedBalanceBase(address user) external view returns (uint256) {
        return depositedBalanceBase[user];
    }

    function getDepositedBalanceQuote(address user) external view returns (uint256) {
        return depositedBalanceQuote[user];
    }

    function getFreeBalanceBase(address user) external view returns (uint256) {
        return freeBalanceBase[user];
    }

    function getFreeBalanceQuote(address user) external view returns (uint256) {
        return freeBalanceQuote[user];
    }

    function getLockedBalanceBase(address user) external view returns (uint256) {
        return lockedBalanceBase[user];
    }

    function getLockedBalanceQuote(address user) external view returns (uint256) {
        return lockedBalanceQuote[user];
    }

    function getOffchainSigner() external view returns (address) {
        return offchainSigner;
    }

    function getLatestStateRootHash() public view returns (bytes32) {
        return latestStateRootHash;
    }
    
    function setLatestStateRootHash(bytes32 nextStateRootHash) external onlyJobManager() {
        latestStateRootHash = nextStateRootHash;
    }

    function deposit(uint256 base_amount, uint256 quote_amount) external {
        require(baseToken.transferFrom(msg.sender, address(this), base_amount), "Transfer failed");
        require(quoteToken.transferFrom(msg.sender, address(this), quote_amount), "Transfer failed");

        depositedBalanceBase[msg.sender] += base_amount;
        depositedBalanceQuote[msg.sender] += quote_amount;

        emit Deposit(msg.sender, base_amount, quote_amount);
    }

    function _receiveResult(bytes32 jobID, bytes memory result) internal override  {
        ClobProgramOutput memory clobResult = abi.decode(result, (ClobProgramOutput));

        // TODO (Maanav): Figure out how to generalize this state root check etc.
        // [ref]: https://github.com/Ethos-Works/InfinityVM/issues/178
        bytes memory encodedBatchInput = getProgramInputsForJob(jobID);
        ClobProgramInput memory batchInput = abi.decode(encodedBatchInput, (ClobProgramInput));
        require(batchInput.previousStateHash == latestStateRootHash, "Invalid state hash passed as input");

        // Update the state root hash
        latestStateRootHash = clobResult.nextStateRootHash;

        // Apply the deposit deltas
        for (uint256 i = 0; i < clobResult.depositDeltas.length; i++) {
           address user = clobResult.depositDeltas[i].user;
           depositedBalanceBase[user] -= clobResult.depositDeltas[i].baseDelta;
           depositedBalanceQuote[user] -= clobResult.depositDeltas[i].quoteDelta;
        }

        // Apply the order deltas
        for (uint256 i = 0; i < clobResult.orderDeltas.length; i++) {
           address user = clobResult.orderDeltas[i].user;
           freeBalanceBase[user] = applyDelta(freeBalanceBase[user], clobResult.orderDeltas[i].freeBaseDelta);
           lockedBalanceBase[user] = applyDelta(lockedBalanceBase[user], clobResult.orderDeltas[i].lockedBaseDelta);
           freeBalanceQuote[user] = applyDelta(freeBalanceQuote[user], clobResult.orderDeltas[i].freeQuoteDelta);
           lockedBalanceQuote[user] = applyDelta(lockedBalanceQuote[user], clobResult.orderDeltas[i].lockedQuoteDelta);
        }

        // Apply the withdraw deltas
        for (uint256 i = 0; i < clobResult.withdrawDeltas.length; i++) {
           address user = clobResult.withdrawDeltas[i].user;
           freeBalanceBase[user] -= clobResult.withdrawDeltas[i].baseDelta;
           freeBalanceQuote[user] -= clobResult.withdrawDeltas[i].quoteDelta;

           require(baseToken.transfer(user, clobResult.withdrawDeltas[i].baseDelta), "Transfer failed");
           require(quoteToken.transfer(user, clobResult.withdrawDeltas[i].quoteDelta), "Transfer failed");
       }
    }

    // @dev Apply an int256 delta to a uint256 value.
    // This function will automatically revert if we try to deduct more 
    // than the current value of a balance.
    function applyDelta(uint256 u, int256 i) public pure returns (uint256) {
        if (i >= 0) {
            // If the int256 is positive, just add it
            return u + uint256(i);
        } else {
            // If the int256 is negative, subtract its absolute value
            return u - uint256(-i);
        }
    }
    
    // Included for EIP-1271. The JobManager calls this function to verify the signature on
    // an offchain job request.
    function isValidSignature(bytes32 messageHash, bytes memory signature) public view override returns (bytes4) {
        address recoveredSigner = ECDSA.tryRecover(messageHash, signature);
        // ClobConsumer allows a single offchainSigner address to sign all offchain job requests
        if (recoveredSigner == offchainSigner) {
            return EIP1271_MAGIC_VALUE;
        } else {
            return INVALID_SIGNATURE;
        }
    }
}
