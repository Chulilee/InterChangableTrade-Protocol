# Margining & Liquidation

Margin accounts, collateral tracking, maintenance margins, and automated
liquidation for leveraged positions in the InterChangableTrade protocol.

## Custody Model

Collateral is held in **real token custody** by the margining contract:

- `deposit_collateral` pulls tokens from the user into the contract via the
  Stellar Asset Contract (`token::Client::transfer`). The user's `require_auth`
  authorizes the transfer. Deposited tokens are tracked in the user's
  `collateral_balances` and in a per-asset `escrowed_collateral` total.
- `withdraw_collateral` returns tokens from the contract to the user, reducing
  both the user's balance and the escrowed total.
- `trigger_liquidation` seizes the liquidated account's entire collateral and
  transfers it to the liquidator. The liquidation incentive is paid from the
  contract's **unallocated reserve** (tokens held in the contract above the
  escrowed total), never from other users' escrowed collateral. The protocol
  funds this reserve by transferring tokens directly to the contract.

Because custody is real, a liquidated user's collateral cannot be
re-withdrawn: their balances are zeroed and the tokens have already moved to
the liquidator.

## State

| Storage key                    | Meaning                                                        |
|--------------------------------|----------------------------------------------------------------|
| `MarginAccount(Address)`       | Per-user collateral balances and open position ids.            |
| `Position(u64)`                | A leveraged position (size, entry/mark price, long/short).     |
| `Config`                       | Protocol config: margin ratios, incentive, price oracle.       |
| `NextPositionId`               | Auto-incrementing position id cursor.                          |
| `EscrowedCollateral(Address)`  | Total tokens of this asset held in escrow by the contract.     |

## Configuration

| Parameter               | Meaning                                                      |
|-------------------------|--------------------------------------------------------------|
| `initial_margin_ratio`  | Minimum collateral to open a position (18-decimal fixed pt). |
| `maintenance_margin_ratio` | Minimum collateral to keep a position open (18-decimal). |
| `liquidation_incentive` | Incentive % paid to liquidators (18-decimal fixed pt).       |
| `price_oracle`          | Address of the price oracle (`get_price(asset)`).            |

## Entry points

| Function                 | Auth        | Description                                                         |
|--------------------------|-------------|---------------------------------------------------------------------|
| `initialize`             | once        | Set protocol configuration.                                         |
| `deposit_collateral`     | user        | Pull tokens into contract custody; credit user's margin account.    |
| `withdraw_collateral`    | user        | Return tokens from custody to user (rejected if below maintenance). |
| `open_position`          | owner       | Open a leveraged position against deposited collateral.             |
| `close_position`         | owner       | Mark a position inactive.                                           |
| `trigger_liquidation`    | liquidator  | Seize collateral, pay incentive, close positions, zero the account. |
| `update_mark_price`      | price oracle| Update a position's mark price.                                     |
| `get_margin_account`     | none (view) | Read a user's margin account.                                       |
| `get_position`           | none (view) | Read a position by id.                                              |
| `get_collateral_value`   | none (view) | Oracle-priced value of an account's collateral.                     |
| `escrowed_collateral`    | none (view) | Total tokens of an asset held in escrow by the contract.            |

## Liquidation flow

1. The liquidator calls `trigger_liquidation(user, liquidator)` and must sign
   (`liquidator.require_auth()`).
2. The contract verifies the account is undercollateralized against the
   maintenance margin (using oracle prices).
3. All active positions are marked inactive.
4. For each collateral asset with a non-zero balance:
   - The full balance is transferred from the contract to the liquidator.
   - `escrowed_collateral` is reduced by the seized amount.
   - The liquidation incentive (`balance * incentive / 1e18`) is paid from the
     contract's unallocated reserve for that asset, capped at what is
     available.
5. The user's `collateral_balances` are zeroed and their position list is
   cleared.
6. A `liq` event is emitted with the oracle-valued incentive actually paid.

## Errors

| Error               | Meaning                                                         |
|---------------------|-----------------------------------------------------------------|
| `NotInitialized`    | Contract has not been initialized.                              |
| `MarginAccountNotFound` | No margin account exists for the user.                        |
| `PositionNotFound`  | No position with the given id.                                  |
| `PositionStillHealthy` | Account meets maintenance margin; cannot be liquidated.       |
| `InsufficientCollateral` | Not enough collateral for the requested withdrawal.           |
| `InsufficientMargin`  | Withdrawal or position would drop below maintenance margin.     |
| `Unauthorized`      | Caller is not authorized for the operation.                     |
| `InvalidAmount`     | Amount is zero or negative.                                     |
| `PriceUnavailable`  | The price oracle could not price a required collateral asset.   |

## Events

| Event                   | Topics                          | Data                  |
|-------------------------|---------------------------------|-----------------------|
| `colldep`               | (user, asset)                   | amount                |
| `collwth`               | (user, asset)                   | amount                |
| `posopen`               | (owner)                         | position              |
| `posclsd`               | (owner, position_id)            | ()                    |
| `liq`                   | (user, liquidator)              | incentive_paid_value  |
| `markprice`             | (position_id)                   | new_mark_price        |
