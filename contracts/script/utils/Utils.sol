// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.24;

import "forge-std/Script.sol";

contract Utils is Script {
    function readOutput(
        string memory outputFileName
    ) internal view returns (string memory) {
        string memory inputDir = string.concat(
            vm.projectRoot(),
            "/script/output/"
        );
        string memory chainDir = string.concat(vm.toString(block.chainid), "/");
        string memory file = string.concat(outputFileName, ".json");
        return vm.readFile(string.concat(inputDir, chainDir, file));
    }

    function writeOutput(
        string memory outputJson,
        string memory outputFileName
    ) internal {
        string memory outputDir = string.concat(
            vm.projectRoot(),
            "/script/output/"
        );
        string memory chainDir = string.concat(vm.toString(block.chainid), "/");
        string memory outputFilePath = string.concat(
            outputDir,
            chainDir,
            outputFileName,
            ".json"
        );
        vm.writeJson(outputJson, outputFilePath);
    }
}
