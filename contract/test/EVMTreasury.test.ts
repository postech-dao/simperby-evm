import { time, loadFixture } from "@nomicfoundation/hardhat-network-helpers";
import { anyValue } from "@nomicfoundation/hardhat-chai-matchers/withArgs";
import { expect } from "chai";
import { ethers, web3, waffle } from "hardhat";
import { Bytes } from "ethers";

const E18n = 10n ** 18n;
const E9n = 10n ** 9n;
const E6n = 10n ** 6n;
const E6_1M = 1_000_000n * E6n;
const E18_1M = 1_000_000n * E18n;
const E18_500K = 500_000n * E18n;

// Below private keys are from hardhat local node
// Do not use these for any personal purposes
const privateKey = [
  "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
  "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
  "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a",
  "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6",
  "0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a",
];

let message: DeliverableMessage;
let data: Bytes;
let data_wrong: Bytes;
let height: Number;
let merkleProof: String;

enum DeliverableMessage {
  FungibleTokenTransfer = 0,
  NonFungibleTokenTransfer = 1,
  Custom = 2,
}

const contract_name: string = "EVM SETTLEMENT CHAIN TREASURY V1";

type UnPromisify<T> = T extends Promise<infer U> ? U : T;

describe("EVMTreasury", function () {
  async function buildFixture() {
    const chain_name = "Ethereum";
    const accounts = await ethers.getSigners();

    const [alice, bob, charlie] = accounts;

    const alice_pk = ethers.utils.computePublicKey(privateKey[0]);
    const alice_pk_modified =
      alice_pk.slice(0, 2) + alice_pk.slice(4, alice_pk.length);
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
        "string",
      ],
      [
        alice_pk_modified, // Delete prefix (02 | 03 | 04)
        [[]],
        ethers.constants.HashZero,
        0,
        1673524666,
        ethers.constants.HashZero,
        ethers.constants.HashZero,
        [alice_pk_modified],
        [100],
        version,
      ]
    );

    const Treasury = await ethers.getContractFactory("EVMTreasury");
    const treasury = await Treasury.deploy(initial_header, chain_name);

    const ERC20Mock = await ethers.getContractFactory("ERC20Mock");
    const erc20 = await ERC20Mock.deploy(
      "TestERC20", // name
      "TST", // symbol
      alice.address, // intial account
      E18_1M // initial balance
    );

    const ERC721Mock = await ethers.getContractFactory("ERC721Mock");
    const erc721 = await ERC721Mock.deploy(
      "TestERC721", // name
      "TST721" // symbol
    );

    return { treasury, erc20, erc721, accounts, alice, bob, charlie };
  }
  let fixture: UnPromisify<ReturnType<typeof buildFixture>>;
  beforeEach(async function () {
    fixture = await loadFixture(buildFixture);
  });

  describe("Deployment", function () {
    it("Should set the right contract name", async function () {
      const { treasury } = fixture;

      expect(await treasury.name()).to.equal(contract_name);
    });
  });

  describe("Update light client", function () {
    let header: any;
    let proof_alice: any;
    let proof_bob: any;
    let proof_charlie: any;
    this.beforeEach(async function () {
      const { treasury, erc20, alice, bob, charlie } = fixture;
      const chain_name = "Ethereum";
      const message = "Second block";

      // Get public key and delete prefix (02 | 03 | 04)
      const alice_pk = ethers.utils.computePublicKey(privateKey[0]);
      const alice_pk_modified =
        alice_pk.slice(0, 2) + alice_pk.slice(4, alice_pk.length);
      const bob_pk = ethers.utils.computePublicKey(privateKey[1]);
      const bob_pk_modified =
        bob_pk.slice(0, 2) + bob_pk.slice(4, bob_pk.length);
      const charlie_pk = ethers.utils.computePublicKey(privateKey[2]);
      const charlie_pk_modified =
        charlie_pk.slice(0, 2) + charlie_pk.slice(4, charlie_pk.length);

      // Make signature and proofs for previous block header
      const prev_header = (await treasury.client()).lastHeader;
      const prev_header_hash = await ethers.utils.keccak256(prev_header);
      const prev_signature = await alice.signMessage(
        ethers.utils.arrayify(prev_header_hash)
      );
      const prev_proof_alice = ethers.utils.defaultAbiCoder.encode(
        ["bytes", "bytes"],
        [alice_pk_modified, prev_signature]
      );

      const version = "0.0.1";

      header = ethers.utils.defaultAbiCoder.encode(
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
          "string",
        ],
        [
          alice_pk_modified,
          [prev_proof_alice],
          prev_header_hash,
          1,
          1673524667,
          ethers.constants.HashZero,
          ethers.constants.HashZero,
          [alice_pk_modified, bob_pk_modified, charlie_pk_modified],
          [100, 100, 100],
          version,
        ]
      );

      // Make signatures for current block header
      const signature_alice = await alice.signMessage(
        ethers.utils.arrayify(await ethers.utils.keccak256(header))
      );
      const signature_bob = await bob.signMessage(
        ethers.utils.arrayify(await ethers.utils.keccak256(header))
      );
      const signature_charlie = await charlie.signMessage(
        ethers.utils.arrayify(await ethers.utils.keccak256(header))
      );

      proof_alice = ethers.utils.defaultAbiCoder.encode(
        ["bytes", "bytes"],
        [alice_pk_modified, signature_alice]
      );
      proof_bob = ethers.utils.defaultAbiCoder.encode(
        ["bytes", "bytes"],
        [bob_pk_modified, signature_bob]
      );
      proof_charlie = ethers.utils.defaultAbiCoder.encode(
        ["bytes", "bytes"],
        [charlie_pk_modified, signature_charlie]
      );
    });
    it("Success case", async function () {
      const { treasury, erc20, alice, bob, charlie } = fixture;

      await expect(
        treasury.updateLightClient(header, [
          proof_alice,
          proof_bob,
          proof_charlie,
        ])
      ).to.emit(treasury, "UpdateLightClient");

      expect((await treasury.client()).lastHeader).to.equal(header);
      // const header_from_contract = (await treasury.client()).last_header;
      // const decoded_header_from_contract = ethers.utils.defaultAbiCoder.decode(
      //   [
      //     "bytes",
      //     "bytes[]",
      //     "bytes32",
      //     "uint64",
      //     "int64",
      //     "bytes32",
      //     "bytes32",
      //     "bytes[]",
      //     "uint64[]",
      //     "string",
      //   ],
      //   header_from_contract
      // );
      // console.log(
      //   "Retrieved header from contract: ",
      //   decoded_header_from_contract
      // );
    });
  });

  // Transfer test case will be updated with proper merkle root type
  describe("Fail case", function () {
    this.beforeEach(async function () {
      const { treasury, erc20, alice } = fixture;

      message = DeliverableMessage.FungibleTokenTransfer;
      data = ethers.utils.defaultAbiCoder.encode(
        ["address", "uint256", "address", "uint256"],
        [erc20.address, E18_500K, alice.address, 1]
      );
      data_wrong = ethers.utils.defaultAbiCoder.encode(
        ["address", "uint256", "address", "uint256"],
        [erc20.address, E18_1M, alice.address, 1]
      );
      height = 0;
      merkleProof = "valid";

      await erc20.connect(alice).transfer(treasury.address, E18_500K);

      expect(await erc20.balanceOf(treasury.address)).to.equal(E18_500K);
    });

    it("Not enough funds", async function () {
      const { treasury, alice } = fixture;

      await expect(
        treasury
          .connect(alice)
          .transferToken(message, data_wrong, height, merkleProof)
      ).to.be.revertedWith("EVMTreasury::withdrawERC20: Insufficient balance");
    });

    it("Not valid merkle proof", async function () {
      const { treasury, alice } = fixture;

      merkleProof = "invalid";
      // height = 1;

      await expect(
        treasury
          .connect(alice)
          .transferToken(message, data, height, merkleProof)
      ).to.be.revertedWith("EVMTreasury::transferToken: Invalid proof");
    });
  });

  describe("Success case", function () {
    this.beforeEach(async function () {
      const { treasury, erc20, erc721, alice } = fixture;

      await erc20.connect(alice).transfer(treasury.address, E18_500K);
      await erc721.connect(alice).mint(treasury.address, 1);

      expect(await erc20.balanceOf(treasury.address)).to.equal(E18_500K);
      expect(await erc721.balanceOf(treasury.address)).to.equal(1);
    });

    it("Transfer ERC20", async function () {
      const { treasury, erc20, alice } = fixture;

      message = DeliverableMessage.FungibleTokenTransfer;
      data = ethers.utils.defaultAbiCoder.encode(
        ["address", "uint256", "address", "uint256"],
        [erc20.address, E18_500K, alice.address, 1]
      );
      height = 0;
      merkleProof = "valid";

      await expect(
        treasury
          .connect(alice)
          .transferToken(message, data, height, merkleProof)
      ).to.emit(treasury, "TransferFungibleToken");

      expect(await erc20.balanceOf(treasury.address)).to.equal(0);
      expect(await erc20.balanceOf(alice.address)).to.equal(E18_1M);
    });

    it("Transfer ERC721", async function () {
      const { treasury, erc721, alice } = fixture;

      message = DeliverableMessage.NonFungibleTokenTransfer;
      data = ethers.utils.defaultAbiCoder.encode(
        ["address", "uint256", "address", "uint256"],
        [erc721.address, 1, alice.address, 1]
      );
      height = 0;
      merkleProof = "valid";

      await expect(
        treasury
          .connect(alice)
          .transferToken(message, data, height, merkleProof)
      ).to.emit(treasury, "TransferNonFungibleToken");

      expect(await erc721.balanceOf(treasury.address)).to.equal(0);
      expect(await erc721.balanceOf(alice.address)).to.equal(1);
    });
  });
});
