import { ethers } from "ethers";

const provider = new ethers.providers.JsonRpcProvider("http://127.0.0.1:8545");

async function GenerateSignature() {
      const jobID = 1;
      const programInputHash = ethers.utils.keccak256(ethers.utils.defaultAbiCoder.encode(['address'], [ethers.constants.AddressZero]));
      const maxCycles = 1000000;
      const programID = "programID";
      const result = ethers.utils.defaultAbiCoder.encode(
        ['address', 'uint256'],
        [ethers.constants.AddressZero,  10]
      );
      
      // Encode the data
      const encodedData = ethers.utils.defaultAbiCoder.encode(
        ['uint32', 'bytes32', 'uint64', 'string', 'bytes'],
        [jobID, programInputHash, maxCycles, programID, result]
      );      
      
      // Sign the message
      const privateKey = ''; // Replace with your actual private key
      const wallet = new ethers.Wallet(privateKey);
      const signature = await wallet.signMessage(ethers.utils.arrayify(encodedData));
      
      console.log("Encoded Data:", encodedData);
      console.log("Signature:", signature);      
   }

GenerateSignature();
