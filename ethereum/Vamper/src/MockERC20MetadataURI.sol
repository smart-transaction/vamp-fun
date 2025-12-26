// SPDX-License-Identifier: MIT
pragma solidity 0.8.33;

contract MockERC20MetadataURI {
    string private _name;
    string private _symbol;
    string private _tokenURI;

    constructor(string memory n, string memory s, string memory u) {
        _name = n;
        _symbol = s;
        _tokenURI = u;
    }

    function name() external view returns (string memory) {
        return _name;
    }

    function symbol() external view returns (string memory) {
        return _symbol;
    }

    // ERC-1046-style
    function tokenURI() external view returns (string memory) {
        return _tokenURI;
    }
}
