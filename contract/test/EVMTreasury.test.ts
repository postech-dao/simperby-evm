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
    const initial_header = "Test";
    const chain_name = "Ethereum";

    const accounts = await ethers.getSigners();
    // alice is the owner of the treasury
    const [alice, bob, charlie] = accounts;

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
          .transfer_token(message, data_wrong, height, merkleProof)
      ).to.be.revertedWith("EVMTreasury::withdrawERC20: Insufficient balance");
    });

    it("Not valid merkle proof", async function () {
      const { treasury, alice } = fixture;

      merkleProof = "invalid";
      // height = 1;

      await expect(
        treasury
          .connect(alice)
          .transfer_token(message, data, height, merkleProof)
      ).to.be.revertedWith("EVMTreasury::transfer_token: Invalid proof");
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
          .transfer_token(message, data, height, merkleProof)
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
          .transfer_token(message, data, height, merkleProof)
      ).to.emit(treasury, "TransferNonFungibleToken");

      expect(await erc721.balanceOf(treasury.address)).to.equal(0);
      expect(await erc721.balanceOf(alice.address)).to.equal(1);
    });
  });
});
