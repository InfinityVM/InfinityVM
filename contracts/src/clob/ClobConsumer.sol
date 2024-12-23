// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {JobManager} from "../coprocessor/JobManager.sol";
import {Consumer} from "../coprocessor/Consumer.sol";
import {StatefulConsumer} from "../coprocessor/StatefulConsumer.sol";
import {OffchainRequester} from "../coprocessor/OffchainRequester.sol";
import {SingleOffchainSigner} from "../coprocessor/SingleOffchainSigner.sol";
import {console} from "forge-std/Script.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {OwnableUpgradeable} from "@openzeppelin-upgrades/contracts/access/OwnableUpgradeable.sol";
import {Initializable} from "@openzeppelin-upgrades/contracts/proxy/utils/Initializable.sol";

contract ClobConsumer is StatefulConsumer, SingleOffchainSigner {
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

    struct ClobResultDeltas {
        DepositDelta[] depositDeltas;
        OrderDelta[] orderDeltas;
        WithdrawDelta[] withdrawDeltas;
    }

    event Deposit(address indexed user, uint256 baseAmount, uint256 quoteAmount);

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

    constructor(IERC20 _baseToken, IERC20 _quoteToken) StatefulConsumer() SingleOffchainSigner() {
        baseToken = _baseToken;
        quoteToken = _quoteToken;
    }

    function initialize(address initialOwner, address jobManager, uint64 initialMaxNonce, bytes32 initialStateRoot, address offchainSigner) public virtual initializer {
        StatefulConsumer.initialize(initialOwner, jobManager, initialMaxNonce, initialStateRoot);
        SingleOffchainSigner.initialize(offchainSigner);
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

    function deposit(uint256 base_amount, uint256 quote_amount) external {
        require(baseToken.transferFrom(msg.sender, address(this), base_amount), "Transfer failed");
        require(quoteToken.transferFrom(msg.sender, address(this), quote_amount), "Transfer failed");

        depositedBalanceBase[msg.sender] += base_amount;
        depositedBalanceQuote[msg.sender] += quote_amount;

        emit Deposit(msg.sender, base_amount, quote_amount);
    }

    function _receiveResult(bytes32 jobID, bytes memory result) internal override  {
        ClobResultDeltas memory deltas = abi.decode(result, (ClobResultDeltas));

        // Apply the deposit deltas
        for (uint256 i = 0; i < deltas.depositDeltas.length; i++) {
           address user = deltas.depositDeltas[i].user;
           depositedBalanceBase[user] -= deltas.depositDeltas[i].baseDelta;
           depositedBalanceQuote[user] -= deltas.depositDeltas[i].quoteDelta;

           freeBalanceBase[user] += deltas.depositDeltas[i].baseDelta;
           freeBalanceQuote[user] += deltas.depositDeltas[i].quoteDelta;
        }

        // Apply the order deltas
        for (uint256 i = 0; i < deltas.orderDeltas.length; i++) {
           address user = deltas.orderDeltas[i].user;
           freeBalanceBase[user] = applyDelta(freeBalanceBase[user], deltas.orderDeltas[i].freeBaseDelta);
           lockedBalanceBase[user] = applyDelta(lockedBalanceBase[user], deltas.orderDeltas[i].lockedBaseDelta);
           freeBalanceQuote[user] = applyDelta(freeBalanceQuote[user], deltas.orderDeltas[i].freeQuoteDelta);
           lockedBalanceQuote[user] = applyDelta(lockedBalanceQuote[user], deltas.orderDeltas[i].lockedQuoteDelta);
        }

        // Apply the withdraw deltas
        for (uint256 i = 0; i < deltas.withdrawDeltas.length; i++) {
           address user = deltas.withdrawDeltas[i].user;
           freeBalanceBase[user] -= deltas.withdrawDeltas[i].baseDelta;
           freeBalanceQuote[user] -= deltas.withdrawDeltas[i].quoteDelta;

           require(baseToken.transfer(user, deltas.withdrawDeltas[i].baseDelta), "Transfer failed");
           require(quoteToken.transfer(user, deltas.withdrawDeltas[i].quoteDelta), "Transfer failed");
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
}
