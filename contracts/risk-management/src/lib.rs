#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod risk_management {
    use ink::storage::Mapping;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum RiskError {
        MarketPaused,
        MaxOrderSizeExceeded,
        CumulativeExposureExceeded,
        Unauthorized,
    }

    #[ink(storage)]
    pub struct RiskManager {
        market_paused: bool,
        max_order_size: Balance,
        cumulative_exposure: Mapping<AccountId, Balance>,
        pauser: AccountId,
    }

    #[ink(event)]
    pub struct MarketPaused {
        #[ink(topic)]
        by: AccountId,
    }

    #[ink(event)]
    pub struct MarketUnpaused {
        #[ink(topic)]
        by: AccountId,
    }

    #[ink(event)]
    pub struct LimitUpdated {
        #[ink(topic)]
        by: AccountId,
        max_order_size: Balance,
    }

    impl RiskManager {
        #[ink(constructor)]
        pub fn new(pauser: AccountId, max_order_size: Balance) -> Self {
            Self {
                market_paused: false,
                max_order_size,
                cumulative_exposure: Mapping::new(),
                pauser,
            }
        }

        #[ink(message)]
        pub fn pause_market(&mut self) -> Result<(), RiskError> {
            if self.env().caller() != self.pauser {
                return Err(RiskError::Unauthorized);
            }
            self.market_paused = true;
            self.env().emit_event(MarketPaused { by: self.env().caller() });
            Ok(())
        }

        #[ink(message)]
        pub fn unpause_market(&mut self) -> Result<(), RiskError> {
            if self.env().caller() != self.pauser {
                return Err(RiskError::Unauthorized);
            }
            self.market_paused = false;
            self.env().emit_event(MarketUnpaused { by: self.env().caller() });
            Ok(())
        }

        #[ink(message)]
        pub fn set_limit(&mut self, max_order_size: Balance) -> Result<(), RiskError> {
            if self.env().caller() != self.pauser {
                return Err(RiskError::Unauthorized);
            }
            self.max_order_size = max_order_size;
            self.env().emit_event(LimitUpdated {
                by: self.env().caller(),
                max_order_size,
            });
            Ok(())
        }

        #[ink(message)]
        pub fn check_limits(&self, order_size: Balance, trader: AccountId) -> Result<(), RiskError> {
            if self.market_paused {
                return Err(RiskError::MarketPaused);
            }
            if order_size > self.max_order_size {
                return Err(RiskError::MaxOrderSizeExceeded);
            }
            let exposure = self.cumulative_exposure.get(&trader).unwrap_or(0);
            if exposure.checked_add(order_size).is_none() {
                return Err(RiskError::CumulativeExposureExceeded);
            }
            Ok(())
        }
    }
}