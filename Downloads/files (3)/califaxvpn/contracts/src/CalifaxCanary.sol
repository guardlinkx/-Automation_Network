// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title CalifaxCanary - Warrant Canary with 24-hour Heartbeat
/// @notice If the operator doesn't chirp within 24 hours, the canary dies
contract CalifaxCanary {
    address public operator;
    uint256 public lastChirp;
    uint256 public chirpInterval;
    string public message;
    bool public alive;

    event Chirped(uint256 timestamp, string message);
    event CanaryDied(uint256 timestamp);
    event OperatorTransferred(address indexed oldOperator, address indexed newOperator);

    modifier onlyOperator() {
        require(msg.sender == operator, "Only operator");
        _;
    }

    constructor(uint256 _chirpIntervalSeconds) {
        operator = msg.sender;
        chirpInterval = _chirpIntervalSeconds;
        lastChirp = block.timestamp;
        alive = true;
        message = "No warrants, subpoenas, or gag orders received.";
    }

    function chirp(string calldata _message) external onlyOperator {
        require(alive, "Canary is dead");
        lastChirp = block.timestamp;
        message = _message;
        emit Chirped(block.timestamp, _message);
    }

    function checkHealth() external view returns (bool isAlive, uint256 lastChirpTime, uint256 secondsSinceChirp, string memory currentMessage) {
        uint256 elapsed = block.timestamp - lastChirp;
        bool healthy = alive && elapsed <= chirpInterval;
        return (healthy, lastChirp, elapsed, message);
    }

    /// @notice Anyone can call this to mark the canary as dead if interval exceeded
    function declareDeathIfStale() external {
        if (alive && block.timestamp - lastChirp > chirpInterval) {
            alive = false;
            emit CanaryDied(block.timestamp);
        }
    }

    function transferOperator(address newOperator) external onlyOperator {
        require(newOperator != address(0), "Invalid operator");
        emit OperatorTransferred(operator, newOperator);
        operator = newOperator;
    }
}
