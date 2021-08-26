Quirks:

1. Have to use multiple describe statements per test file in order to force tests to run synchronously
2. Explain how version works

Ix to make it work
```shell
solana-test-validator --reset

cargo build-bpf
solana program deploy

# airdrop doesn't work on localhost, so have to do it manually
# fund game creator's account
solana transfer AFe99p6byLxYfEV9E1nNumSeKdtgXm2HL5Gy5dN6icj9 10 --allow-unfunded-recipient
# fund alice's account
solana transfer Ga8HG4NzgcYkegLoJDmxJemEU1brewF2XZLNHd6B4wJ7 10 --allow-unfunded-recipient
# fund bob's account
solana transfer BxiV2mYXbBma1Kv7kxnn7cdM93oFHL4BhT9G23hiFfUP 10 --allow-unfunded-recipient 
```