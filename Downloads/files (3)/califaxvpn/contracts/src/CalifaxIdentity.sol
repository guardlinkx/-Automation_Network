// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title CalifaxIdentity - Decentralized Identity Registry on Polygon
/// @notice Manages DIDs for CalifaxVPN users
contract CalifaxIdentity {
    struct Identity {
        string did;
        bytes publicKey;
        uint256 createdAt;
        uint256 updatedAt;
        bool isActive;
    }

    mapping(address => Identity) public identities;
    mapping(string => address) public didToAddress;

    uint256 public totalIdentities;

    event IdentityRegistered(address indexed wallet, string did, uint256 timestamp);
    event IdentityDeactivated(address indexed wallet, string did, uint256 timestamp);
    event IdentityUpdated(address indexed wallet, string did, uint256 timestamp);

    modifier onlyRegistered() {
        require(identities[msg.sender].isActive, "Identity not registered or inactive");
        _;
    }

    function register(string calldata did, bytes calldata publicKey) external {
        require(!identities[msg.sender].isActive, "Already registered");
        require(didToAddress[did] == address(0), "DID already taken");
        require(bytes(did).length > 0, "DID cannot be empty");
        require(publicKey.length > 0, "Public key cannot be empty");

        identities[msg.sender] = Identity({
            did: did,
            publicKey: publicKey,
            createdAt: block.timestamp,
            updatedAt: block.timestamp,
            isActive: true
        });

        didToAddress[did] = msg.sender;
        totalIdentities++;

        emit IdentityRegistered(msg.sender, did, block.timestamp);
    }

    function updatePublicKey(bytes calldata newPublicKey) external onlyRegistered {
        require(newPublicKey.length > 0, "Public key cannot be empty");
        identities[msg.sender].publicKey = newPublicKey;
        identities[msg.sender].updatedAt = block.timestamp;

        emit IdentityUpdated(msg.sender, identities[msg.sender].did, block.timestamp);
    }

    function deactivate() external onlyRegistered {
        Identity storage id = identities[msg.sender];
        id.isActive = false;
        id.updatedAt = block.timestamp;

        emit IdentityDeactivated(msg.sender, id.did, block.timestamp);
    }

    function resolve(string calldata did) external view returns (address wallet, bytes memory publicKey, uint256 createdAt, bool isActive) {
        address addr = didToAddress[did];
        require(addr != address(0), "DID not found");
        Identity storage id = identities[addr];
        return (addr, id.publicKey, id.createdAt, id.isActive);
    }

    function getIdentity(address wallet) external view returns (string memory did, bytes memory publicKey, uint256 createdAt, bool isActive) {
        Identity storage id = identities[wallet];
        return (id.did, id.publicKey, id.createdAt, id.isActive);
    }
}
