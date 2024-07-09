// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {JobManager} from "../src/JobManager.sol";
import {Consumer} from "../src/Consumer.sol";
import {MockConsumer} from "./mocks/MockConsumer.sol";

contract CoprocessorTest is Test {
    JobManager public jobManager;
    MockConsumer public consumer;

    function setUp() public {
        jobManager = new JobManager(address(0), address(1));
        consumer = new MockConsumer(address(jobManager));
    }

    function test_JobManager_CreateJob() public {
        uint32 jobID = jobManager.createJob("programID", "programInput");
        assertEq(jobID, 1);
        JobManager.JobMetadata memory jobMetadata = jobManager.getJobMetadata(jobID);
        assertEq(jobMetadata.programID, "programID");
        assertEq(jobMetadata.caller, address(this));
        assertEq(jobMetadata.status, 1);
    }

    function test_Consumer_RequestJob() public {
        uint32 jobID = consumer.requestBalance(address(0));
        assertEq(jobID, 1);
        assertEq(consumer.getProgramInputsForJob(jobID), abi.encode(address(0)));
        JobManager.JobMetadata memory jobMetadata = jobManager.getJobMetadata(jobID);
        assertEq(jobMetadata.programID, "programID");
        assertEq(jobMetadata.caller, address(consumer));
        assertEq(jobMetadata.status, 1);
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

    function test_JobManager_CancelJobByConsumer() public {
        test_Consumer_RequestJob();
        vm.prank(address(consumer));
        jobManager.cancelJob(1);
        JobManager.JobMetadata memory jobMetadata = jobManager.getJobMetadata(1);
        assertEq(jobMetadata.status, 2);
    }

    function test_JobManager_CancelJobByOwner() public {
        test_Consumer_RequestJob();
        vm.prank(jobManager.owner());
        jobManager.cancelJob(1);
        JobManager.JobMetadata memory jobMetadata = jobManager.getJobMetadata(1);
        assertEq(jobMetadata.status, 2);
    }

    function test_Consumer_GetBalance() public {
        assertEq(consumer.getBalance(address(0)), 0);
    }
}
