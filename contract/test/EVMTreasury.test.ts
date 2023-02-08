import { time, loadFixture } from "@nomicfoundation/hardhat-network-helpers";
import { anyValue } from "@nomicfoundation/hardhat-chai-matchers/withArgs";
import { expect } from "chai";
import { ethers, web3, waffle } from "hardhat";
import { Bytes } from "ethers";
import {
  initialHeader,
  nextHeader,
  fp,
  tx,
  merkleProof,
} from "../scripts/misc/constants";

const E18n = 10n ** 18n;
const E9n = 10n ** 9n;
const E6n = 10n ** 6n;
const E6_1M = 1_000_000n * E6n;
const E18_1M = 1_000_000n * E18n;
const E18_500K = 500_000n * E18n;

const contract_name: string = "EVM SETTLEMENT CHAIN TREASURY V1";

type UnPromisify<T> = T extends Promise<infer U> ? U : T;

// @dev: This test is not working properly since we need to link it with simperby.
// Simperby block header, transaction and other types should be updated to do the following test.

describe("EVMTreasury", function () {
  async function buildFixture() {
    const accounts = await ethers.getSigners();

    const [alice, bob, charlie] = accounts;

    const Treasury = await ethers.getContractFactory("EVMTreasury");
    const treasury = await Treasury.deploy(initialHeader);

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

  describe("Update light client - fail", function () {
    it("Success case", async function () {
      const { treasury, erc20, alice, bob, charlie } = fixture;

      // await expect(treasury.updateLightClient(nextHeader, fp)).to.emit(
      //   treasury,
      //   "UpdateLightClient"
      // );

      // expect((await treasury.lightClient()).lastHeader).to.equal(nextHeader);

      // After change to uncompressed public key type, it will be fixed.
      await expect(
        treasury.updateLightClient(nextHeader, fp)
      ).to.be.revertedWith(
        "Verify::verifyHeaderToHeader: Invalid block author"
      );
    });
  });

  /// After fixing tx format to bincode, it will be fixed.
  /// For now, skip the test.

  // describe("Transfer fail case", function () {
  //   this.beforeEach(async function () {
  //     const { treasury, erc20, alice } = fixture;

  //     height = 1;

  //     await erc20.connect(alice).transfer(treasury.address, E18_500K);

  //     expect(await erc20.balanceOf(treasury.address)).to.equal(E18_500K);
  //   });

  //   it("Not valid merkleProof", async function () {
  //     const { treasury, alice } = fixture;

  //     await expect(
  //       treasury.execute(tx, height, merkleProof)
  //     ).to.be.revertedWith("EVMTreasury::withdrawERC20: Insufficient balance");
  //   });
  // });

  // describe("Success case", function () {
  //   this.beforeEach(async function () {
  //     const { treasury, erc20, erc721, alice } = fixture;

  //     await erc20.connect(alice).transfer(treasury.address, E18_500K);
  //     await erc721.connect(alice).mint(treasury.address, 1);

  //     expect(await erc20.balanceOf(treasury.address)).to.equal(E18_500K);
  //     expect(await erc721.balanceOf(treasury.address)).to.equal(1);
  //   });

  //   it("Transfer ERC20", async function () {
  //     const { treasury, erc20, alice } = fixture;

  //     message = DeliverableMessage.FungibleTokenTransfer;
  //     data = ethers.utils.defaultAbiCoder.encode(
  //       ["address", "uint256", "address", "uint256"],
  //       [erc20.address, E18_500K, alice.address, 1]
  //     );
  //     height = 0;
  //     merkleProof = "valid";

  //     await expect(
  //       treasury
  //         .connect(alice)
  //         .transferToken(message, data, height, merkleProof)
  //     ).to.emit(treasury, "TransferFungibleToken");

  //     expect(await erc20.balanceOf(treasury.address)).to.equal(0);
  //     expect(await erc20.balanceOf(alice.address)).to.equal(E18_1M);
  //   });

  //   it("Transfer ERC721", async function () {
  //     const { treasury, erc721, alice } = fixture;

  //     message = DeliverableMessage.NonFungibleTokenTransfer;
  //     data = ethers.utils.defaultAbiCoder.encode(
  //       ["address", "uint256", "address", "uint256"],
  //       [erc721.address, 1, alice.address, 1]
  //     );
  //     height = 0;
  //     merkleProof = "valid";

  //     await expect(
  //       treasury
  //         .connect(alice)
  //         .transferToken(message, data, height, merkleProof)
  //     ).to.emit(treasury, "TransferNonFungibleToken");

  //     expect(await erc721.balanceOf(treasury.address)).to.equal(0);
  //     expect(await erc721.balanceOf(alice.address)).to.equal(1);
  //   });
  // });
});
