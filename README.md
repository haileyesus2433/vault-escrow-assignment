# Vault and Escrow Programs

Solana programs built with Anchor for the Turbin3 assignment. Contains two programs: a SOL vault and a token escrow.

## Programs

### Vault

A non-custodial SOL vault where users can deposit, withdraw, and close their personal vault.

**Architecture:**
- `VaultState` PDA (`[b"state", user]`) stores bumps for cheap re-derivation
- `vault` PDA (`[b"vault", vault_state]`) holds the SOL balance

**Instructions:**
| Instruction | Description |
|-------------|-------------|
| `initialize` | Creates the `VaultState` and `vault` accounts, saves bumps |
| `deposit` | Transfers SOL from user to vault via CPI to System Program |
| `withdraw` | Transfers SOL from vault to user via PDA-signed CPI |
| `close` | Drains vault balance then closes `VaultState`, returning rent to user |

### Escrow

A trustless token swap escrow. The maker deposits Token A and specifies how much Token B they want. A taker can fulfill the trade atomically, or the maker can cancel and reclaim their tokens.

**Architecture:**
- `Escrow` PDA (`[b"escrow", maker, seed]`) stores trade parameters; `seed: u64` allows one maker to open multiple escrows
- `vault` ATA owned by the `Escrow` PDA holds the deposited Token A

**Instructions:**
| Instruction | Description |
|-------------|-------------|
| `make` | Initializes escrow, creates vault ATA, deposits Token A |
| `take` | Atomic swap: transfers Token B to maker, Token A to taker, closes vault and escrow |
| `refund` | Returns Token A to maker, closes vault and escrow |
| `update` | Updates the `receive` amount before the escrow is taken |

## Tests

All tests are written in Rust using [LiteSVM](https://github.com/LiteSVM/litesvm) for fast, in-process program testing.

```bash
# Run vault tests
cargo test -p vault --test test_vault

# Run escrow tests
cargo test -p escrow --test test_escrow
```

**Vault test:** `test_initialize_deposit_withdraw_close` - full lifecycle

**Escrow tests:**
- `test_make_and_take` - happy path atomic swap
- `test_make_and_refund` - maker cancels and reclaims tokens

## Build

```bash
anchor build
```

Requires Rust, Anchor CLI, and the Solana toolchain.
