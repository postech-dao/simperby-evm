# Simperby-evm

[![License: MIT](https://img.shields.io/github/license/postech-dao/simperby)](https://opensource.org/licenses/MIT)
[![Twitter][twitter-image]][twitter-link]

[twitter-image]: https://img.shields.io/twitter/follow/postech_dao?style=social
[twitter-link]: https://twitter.com/postech_dao

This repository implements the settlement chain interface for an EVM-based ecosystem that can interact with simperby.
If you don't know what simperby is, please visit [simperby]("https://github.com/postech-dao/simperby").

## Building

1. `git clone https://github.com/postech-dao/simperby-evm.git`
2. `npm install`
3. Install [foundry](https://book.getfoundry.sh/getting-started/installation) if you want to use for testing.
4. `forge install`
5. (Optional) Create .env and fill your environment variables with the format of .env.example.

## Testing

Currently, we support both 'hardhat' and 'foundry' for testing.

### Hardhat

1. Install dependencies
2. `npx hardhat compile`
3. `npx hardhat node`
4. `npx hardhat test --network localhost`

You can check build results in `/artifacts` and gas reports in `/contract`.

### Foundry

1. set up foundry with the above instructions.
2. `forge build`
3. `forge test` or `forge test -vv` if you want to check logs.
4. `anvil` if you want to run a local node with foundry.

For more details about foundry, please refer to [foundry]("https://github.com/foundry-rs/foundry")

## Deployment

If you want to deploy EVMTreasury to any EVM chains, you need to fill `.env` file and check `hardhat.config.ts`.

1. `npx hardhat compile`
2. `npx hardhat run --network <network> scripts/_01_deploy_treasury.sol`.
3. You must check your initial block header is properly set with `initialHeader` variable in `misc/constants.ts`.
4. Update `misc/addresses.ts` with deployed contract address.

## Misc

You can use prettier for code formatting.

```bash
npx prettier --write .
```

## License

This project is licensed under the [MIT license](./LICENSE).
