import { network, ethers, web3 } from "hardhat";
import { SignerWithAddress } from "@nomiclabs/hardhat-ethers/signers";

async function main() {
  const initial_header = "Test";
  const chain_name = "Ethereum";

  const signers = await ethers.getSigners();

  const owner = signers[0];

  console.log("Deploying contracts with the account:", owner.address);

  const Treasury = await ethers.getContractFactory("EVMTreasury");
  const treasury = await Treasury.deploy(initial_header, chain_name);

  await treasury.deployed();

  console.log(
    `EVM Treasury of ${chain_name} deployed at ${treasury.address} successfully`,
  );
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch(error => {
  console.error(error);
  process.exitCode = 1;
});
