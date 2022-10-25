// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity >=0.6.0;

import {IInterchainSecurityModule} from "./IInterchainSecurityModule.sol";

interface IMultisigModule is IInterchainSecurityModule {
    function threshold(uint32 _domain) external view returns (uint256);

    // TODO: Should this be bytes32[]?
    function validators(uint32 _domain)
        external
        view
        returns (address[] memory);
}