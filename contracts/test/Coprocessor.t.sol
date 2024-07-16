// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {JobManager} from "../src/JobManager.sol";
import {Consumer} from "../src/Consumer.sol";
import {MockConsumer} from "./mocks/MockConsumer.sol";
import {CoprocessorDeployer} from "../script/CoprocessorDeployer.s.sol";

contract CoprocessorTest is Test, CoprocessorDeployer {
    uint64 DEFAULT_MAX_CYCLES = 1_000_000;
    address RELAYER = address(1);
    address COPROCESSOR_OPERATOR = 0x184c47137933253f49325B851307Ab1017863BD0;

    event JobCreated(uint32 indexed jobID, uint64 maxCycles, bytes indexed programID, bytes programInput);
    event JobCancelled(uint32 indexed jobID);
    event JobCompleted(uint32 indexed jobID, bytes result);

    function setUp() public {
        deployCoprocessorContracts(RELAYER, COPROCESSOR_OPERATOR);
    }

    function test_JobManager_CreateJob() public {
        vm.expectEmit(true, true, true, true);
        emit JobCreated(1, DEFAULT_MAX_CYCLES, "programID", "programInput");
        uint32 jobID = jobManager.createJob("programID", "programInput", DEFAULT_MAX_CYCLES);
        assertEq(jobID, 1);
        JobManager.JobMetadata memory jobMetadata = jobManager.getJobMetadata(jobID);
        assertEq(jobMetadata.programID, "programID");
        assertEq(jobMetadata.maxCycles, DEFAULT_MAX_CYCLES);
        assertEq(jobMetadata.caller, address(this));
        assertEq(jobMetadata.status, 1);
    }

    function test_Consumer_RequestJob() public {
        vm.expectEmit(true, true, true, true);
        emit JobCreated(1, DEFAULT_MAX_CYCLES, "programID", abi.encode(address(0)));
        uint32 jobID = consumer.requestBalance("programID", address(0));
        assertEq(jobID, 1);
        assertEq(consumer.getProgramInputsForJob(jobID), abi.encode(address(0)));
        JobManager.JobMetadata memory jobMetadata = jobManager.getJobMetadata(jobID);
        assertEq(jobMetadata.programID, "programID");
        assertEq(jobMetadata.maxCycles, DEFAULT_MAX_CYCLES);
        assertEq(jobMetadata.caller, address(consumer));
        assertEq(jobMetadata.status, 1);
    }

        function test_JobManager_CancelJobByConsumer() public {
        test_Consumer_RequestJob();
        vm.expectEmit(true, false, false, false);
        emit JobCancelled(1);
        vm.prank(address(consumer));
        jobManager.cancelJob(1);
        JobManager.JobMetadata memory jobMetadata = jobManager.getJobMetadata(1);
        assertEq(jobMetadata.status, 2);
    }

    function test_JobManager_CancelJobByOwner() public {
        test_Consumer_RequestJob();
        vm.expectEmit(true, false, false, false);
        emit JobCancelled(1);
        vm.prank(jobManager.owner());
        jobManager.cancelJob(1);
        JobManager.JobMetadata memory jobMetadata = jobManager.getJobMetadata(1);
        assertEq(jobMetadata.status, 2);
    }

    function testRevertWhen_JobManager_CancelJobUnauthorized() public {
        test_JobManager_CreateJob();
        vm.prank(address(1));
        vm.expectRevert("JobManager.cancelJob: caller is not the job creator or JobManager owner");
        jobManager.cancelJob(1);
    }

    function testRevertWhen_JobManager_CancelJobNotPending() public {
        test_Consumer_RequestJob();
        vm.prank(address(consumer));
        jobManager.cancelJob(1);
        vm.prank(address(consumer));
        vm.expectRevert("JobManager.cancelJob: job is not in pending state");
        jobManager.cancelJob(1);
    }

    function testRevertWhen_Consumer_ReceiveResultUnauthorized() public {
        test_Consumer_RequestJob();
        vm.prank(address(1));
        vm.expectRevert("Consumer.onlyJobManager: caller is not the job manager");
        consumer.receiveResult(1, abi.encode(address(0)));
    }

    function test_JobManager_SubmitResult() public {
        test_Consumer_RequestJob();

        // Generated using rust/crates/scripts/signer.rs
        bytes memory resultWithMetadata = hex"0000000000000000000000000000000000000000000000000000000000000001290decd9548b62a8d60345a988386fc84ba6bc95484008f6362f93160ef3e56300000000000000000000000000000000000000000000000000000000000f424000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e0000000000000000000000000000000000000000000000000000000000000000970726f6772616d4944000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a";
        bytes memory signature = hex"88db44d83f6d32ff87647d9ac8d468b74ac6afdbc76f4ee7cc9260f93e3e48c9617f4ed3e7088e529a78c481fa9d58affb166dbb388e300e42c3de4e7b54d6091b";

        vm.expectEmit(true, true, false, false);
        emit JobCompleted(1, abi.encode(address(0), 10));
        vm.prank(RELAYER);
        jobManager.submitResult(resultWithMetadata, signature);

        JobManager.JobMetadata memory jobMetadata = jobManager.getJobMetadata(1);
        // Check that job status is COMPLETED
        assertEq(jobMetadata.status, 3);

        // Check that state was correctly updated in Consumer contract
        assertEq(consumer.getBalance(address(0)), 10);
        assertEq(consumer.getJobResult(1), abi.encode(address(0), 10));
    }

    function testRevertWhen_JobManager_SubmitResultUnauthorized() public {
        test_Consumer_RequestJob();
        vm.prank(address(2));
        vm.expectRevert("JobManager.submitResult: caller is not the relayer");
        jobManager.submitResult(abi.encode("resultWithMetadata"), abi.encodePacked("signature"));
    }

    function testRevertWhen_JobManager_SubmitResultInvalidSignature() public {
        test_Consumer_RequestJob();

        bytes memory resultWithMetadata = hex"0000000000000000000000000000000000000000000000000000000000000001290decd9548b62a8d60345a988386fc84ba6bc95484008f6362f93160ef3e56300000000000000000000000000000000000000000000000000000000000f424000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e0000000000000000000000000000000000000000000000000000000000000000970726f6772616d4944000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a";
        bytes memory signature = hex"89db44d83f6d32ff87647d9ac8d468b74ac6afdbc76f4ee7cc9260f93e3e48c9617f4ed3e7088e529a78c481fa9d58affb166dbb388e300e42c3de4e7b54d6091b";

        vm.expectRevert("JobManager.submitResult: Invalid signature");
        vm.prank(RELAYER);
        jobManager.submitResult(resultWithMetadata, signature);
    }

    function testRevertWhen_JobManager_SubmitResultCancelledJob() public {
        test_JobManager_CancelJobByConsumer();

        bytes memory resultWithMetadata = hex"0000000000000000000000000000000000000000000000000000000000000001290decd9548b62a8d60345a988386fc84ba6bc95484008f6362f93160ef3e56300000000000000000000000000000000000000000000000000000000000f424000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e0000000000000000000000000000000000000000000000000000000000000000970726f6772616d4944000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a";
        bytes memory signature = hex"88db44d83f6d32ff87647d9ac8d468b74ac6afdbc76f4ee7cc9260f93e3e48c9617f4ed3e7088e529a78c481fa9d58affb166dbb388e300e42c3de4e7b54d6091b";

        vm.expectRevert("JobManager.submitResult: job is not in pending state");
        vm.prank(RELAYER);
        jobManager.submitResult(resultWithMetadata, signature);
    }

    function testRevertWhen_JobManager_SubmitResultWrongProgramID() public {
        test_Consumer_RequestJob();

        bytes memory resultWithMetadata = hex"0000000000000000000000000000000000000000000000000000000000000001290decd9548b62a8d60345a988386fc84ba6bc95484008f6362f93160ef3e56300000000000000000000000000000000000000000000000000000000000f424000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e0000000000000000000000000000000000000000000000000000000000000000c70726f6772616d4944313233000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a";
        bytes memory signature = hex"1e84e7839672b237541a76feac1ba35e179ab8a325f4c66ca135d7c5ebea3061525e718e2e5f0a5a9803d0e4717bbbad990376874312db3f0db5ce2bed6980ef1b";

        vm.expectRevert("JobManager.submitResult: program ID signed by coprocessor doesn't match program ID submitted with job");
        vm.prank(RELAYER);
        jobManager.submitResult(resultWithMetadata, signature);
    }

    function testRevertWhen_JobManager_SubmitResultWrongProgramInputHash() public {
        test_Consumer_RequestJob();

        bytes memory resultWithMetadata = hex"0000000000000000000000000000000000000000000000000000000000000001e570eff78be1b11cf36ef150c7ed13e3fa520033b0a14059887fc69332fb4c3300000000000000000000000000000000000000000000000000000000000f424000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e0000000000000000000000000000000000000000000000000000000000000000970726f6772616d4944000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a";
        bytes memory signature = hex"599c86688b587cfdd46d187e2a6ccac8aba11d546e3844846ffd9e7ac008ddc84c136b66eda5d1ff9bf440cde2c2793f905807ffd57722fb1db8bdbef74eed1f1b";

        vm.expectRevert("JobManager.submitResult: program input signed by coprocessor doesn't match program input submitted with job");
        vm.prank(RELAYER);
        jobManager.submitResult(resultWithMetadata, signature);
    }
}
