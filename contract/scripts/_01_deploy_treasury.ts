import { network, ethers, web3 } from "hardhat";
import { SignerWithAddress } from "@nomiclabs/hardhat-ethers/signers";

async function main() {
  const chain_name = "Ethereum";
  // author_genesis public key needs to be changed.
  const accounts = await ethers.getSigners();
  // alice is the owner of the treasury
  const [alice, bob, charlie] = accounts;
  // make public key from alice's private key
  const alice_pk = ethers.utils.computePublicKey(
    "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"
  );
  const alice_pk_modified =
    alice_pk.slice(0, 2) + alice_pk.slice(4, alice_pk.length);
  // const genesis_signature = await alice.signMessage(message);
  // const proof = ethers.utils.defaultAbiCoder.encode(
  //   ["bytes", "bytes"],
  //   [alice_pk, genesis_signature]
  // );
  const version = "0.0.1";

  const initial_header = ethers.utils.defaultAbiCoder.encode(
    [
      "bytes",
      "bytes[]",
      "bytes32",
      "uint64",
      "int64",
      "bytes32",
      "bytes32",
      "bytes[]",
      "uint64[]",
      "bytes32",
    ],
    [
      alice_pk_modified,
      [[]],
      ethers.constants.HashZero,
      0,
      1673524666,
      ethers.constants.HashZero,
      ethers.constants.HashZero,
      [alice_pk_modified],
      [100],
      ethers.utils.formatBytes32String(version),
    ]
  );

  const signers = await ethers.getSigners();

  const owner = signers[0];

  console.log("Deploying contracts with the account:", owner.address);

  const Treasury = await ethers.getContractFactory("EVMTreasury");
  const treasury = await Treasury.deploy(initial_header, chain_name);

  await treasury.deployed();

  console.log(
    `EVM Treasury of ${chain_name} deployed at ${treasury.address} successfully`
  );
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
