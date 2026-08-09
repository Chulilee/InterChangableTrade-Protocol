use soroban_sdk::{contracttype, Address, Env, Map, Vec};

/// Represents a net transfer obligation between a debtor and creditor for a specific asset.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetObligation {
    pub debtor: Address,
    pub creditor: Address,
    pub asset: Address,
    pub amount: i128,
}

pub struct NettingEngine;

impl NettingEngine {
    /// Computes net transfer obligations for a batch of trades.
    /// Returns a tuple containing the vector of netting obligations and the original transfer count.
    pub fn compute_net_obligations(
        env: &Env,
        trades: &Vec<crate::Trade>,
    ) -> (Vec<NetObligation>, u32) {
        // Track net balance per (party, asset).
        // Positive balance = creditor (receives token).
        // Negative balance = debtor (pays token).
        let mut net_balances: Map<(Address, Address), i128> = Map::new(env);
        let mut original_transfer_count: u32 = 0;

        for trade in trades.iter() {
            // Each trade natively requires 2 transfers:
            // 1. Seller -> Buyer for base_asset (base_amount)
            // 2. Buyer -> Seller for quote_asset (quote_amount)
            original_transfer_count += 2;

            // Buyer balance updates
            let buyer_base_key = (trade.buyer.clone(), trade.base_asset.clone());
            let current = net_balances.get(buyer_base_key.clone()).unwrap_or(0);
            net_balances.set(buyer_base_key, current + trade.base_amount);

            let buyer_quote_key = (trade.buyer.clone(), trade.quote_asset.clone());
            let current = net_balances.get(buyer_quote_key.clone()).unwrap_or(0);
            net_balances.set(buyer_quote_key, current - trade.quote_amount);

            // Seller balance updates
            let seller_base_key = (trade.seller.clone(), trade.base_asset.clone());
            let current = net_balances.get(seller_base_key.clone()).unwrap_or(0);
            net_balances.set(seller_base_key, current - trade.base_amount);

            let seller_quote_key = (trade.seller.clone(), trade.quote_asset.clone());
            let current = net_balances.get(seller_quote_key.clone()).unwrap_or(0);
            net_balances.set(seller_quote_key, current + trade.quote_amount);
        }

        // Collect unique assets involved in the batch
        let mut assets: Vec<Address> = Vec::new(env);
        for key in net_balances.keys().iter() {
            let asset = key.1;
            if !assets.contains(&asset) {
                assets.push_back(asset);
            }
        }

        let mut obligations: Vec<NetObligation> = Vec::new(env);

        // For each asset, pair debtors (net < 0) with creditors (net > 0)
        for asset in assets.iter() {
            let mut debtors: Vec<(Address, i128)> = Vec::new(env);
            let mut creditors: Vec<(Address, i128)> = Vec::new(env);

            for key in net_balances.keys().iter() {
                if key.1 == asset {
                    let bal = net_balances.get(key.clone()).unwrap();
                    if bal < 0 {
                        debtors.push_back((key.0, -bal));
                    } else if bal > 0 {
                        creditors.push_back((key.0, bal));
                    }
                }
            }

            let mut d_idx: u32 = 0;
            let mut c_idx: u32 = 0;

            while d_idx < debtors.len() && c_idx < creditors.len() {
                let (debtor_addr, debt_amt) = debtors.get(d_idx).unwrap();
                let (creditor_addr, cred_amt) = creditors.get(c_idx).unwrap();

                let transfer_amt = if debt_amt < cred_amt {
                    debt_amt
                } else {
                    cred_amt
                };

                if transfer_amt > 0 {
                    obligations.push_back(NetObligation {
                        debtor: debtor_addr.clone(),
                        creditor: creditor_addr.clone(),
                        asset: asset.clone(),
                        amount: transfer_amt,
                    });
                }

                let remaining_debt = debt_amt - transfer_amt;
                let remaining_cred = cred_amt - transfer_amt;

                if remaining_debt == 0 {
                    d_idx += 1;
                } else {
                    debtors.set(d_idx, (debtor_addr, remaining_debt));
                }

                if remaining_cred == 0 {
                    c_idx += 1;
                } else {
                    creditors.set(c_idx, (creditor_addr, remaining_cred));
                }
            }
        }

        (obligations, original_transfer_count)
    }
}
