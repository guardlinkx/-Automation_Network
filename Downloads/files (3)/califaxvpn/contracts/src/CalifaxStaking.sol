// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title CalifaxStaking - Node Operator Staking with Slashing
/// @notice Operators stake tokens to run relay/exit nodes. Bad behavior = slashing.
contract CalifaxStaking {
    address public admin;
    uint256 public minimumStake;
    uint256 public slashPercent; // basis points (100 = 1%)

    struct Stake {
        uint256 amount;
        uint256 stakedAt;
        bool isActive;
        string nodeEndpoint;
        string region;
        uint256 slashedAmount;
    }

    mapping(address => Stake) public stakes;
    address[] public operators;
    uint256 public totalStaked;

    event Staked(address indexed operator, uint256 amount, string endpoint, string region);
    event Unstaked(address indexed operator, uint256 amount);
    event Slashed(address indexed operator, uint256 amount, string reason);

    modifier onlyAdmin() {
        require(msg.sender == admin, "Only admin");
        _;
    }

    constructor(uint256 _minimumStake, uint256 _slashPercent) {
        admin = msg.sender;
        minimumStake = _minimumStake;
        slashPercent = _slashPercent;
    }

    function stake(string calldata endpoint, string calldata region) external payable {
        require(msg.value >= minimumStake, "Below minimum stake");
        require(!stakes[msg.sender].isActive, "Already staking");

        stakes[msg.sender] = Stake({
            amount: msg.value,
            stakedAt: block.timestamp,
            isActive: true,
            nodeEndpoint: endpoint,
            region: region,
            slashedAmount: 0
        });
        operators.push(msg.sender);
        totalStaked += msg.value;

        emit Staked(msg.sender, msg.value, endpoint, region);
    }

    function unstake() external {
        Stake storage s = stakes[msg.sender];
        require(s.isActive, "Not staking");
        require(block.timestamp > s.stakedAt + 7 days, "Must stake for at least 7 days");

        uint256 refund = s.amount - s.slashedAmount;
        s.isActive = false;
        totalStaked -= s.amount;

        (bool sent, ) = payable(msg.sender).call{value: refund}("");
        require(sent, "Transfer failed");

        emit Unstaked(msg.sender, refund);
    }

    function slash(address operator_addr, string calldata reason) external onlyAdmin {
        Stake storage s = stakes[operator_addr];
        require(s.isActive, "Operator not staking");

        uint256 penalty = (s.amount * slashPercent) / 10000;
        s.slashedAmount += penalty;

        emit Slashed(operator_addr, penalty, reason);
    }

    function getOperatorCount() external view returns (uint256) {
        uint256 count = 0;
        for (uint256 i = 0; i < operators.length; i++) {
            if (stakes[operators[i]].isActive) count++;
        }
        return count;
    }

    function getOperatorInfo(address op) external view returns (uint256 amount, string memory endpoint, string memory region, bool isActive) {
        Stake storage s = stakes[op];
        return (s.amount, s.nodeEndpoint, s.region, s.isActive);
    }

    receive() external payable {}
}
