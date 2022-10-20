#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod workshop {
    use ink_lang::utils::initialize_contract;
    use ink_storage::traits::SpreadAllocate;
    use ink_storage::Mapping;

    #[ink(event)]
    pub struct Deposited {
        from: AccountId,
        balance: u128,
    }

    #[ink(event)]
    pub struct Withdrawn {
        to: AccountId,
        balance: u128,
    }

    #[derive(PartialEq, Debug, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ContractError {
        AccountWithoutBalance,
        InsufficientFunds,
        ExpectedWithdrawalAmountExceedsAccountBalance,
        WithdrawTransferFailed,
    }

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct Workshop {
        balances: Mapping<AccountId, u128>,
    }

    impl Workshop {
        #[ink(constructor)]
        pub fn new() -> Self {
            initialize_contract(|contract: &mut Self| {
                contract.balances = <Mapping<AccountId, u128>>::default();
            })
        }

        #[ink(message)]
        pub fn get_balance_by_account(&self) -> Result<u128, ContractError> {
            let caller = self.get_caller();

            match self.balances.get(caller) {
                Some(account_balance) => Ok(account_balance),
                None => Err(ContractError::AccountWithoutBalance),
            }
        }

        #[ink(message, payable)]
        pub fn deposit(&mut self) -> Result<(), ContractError> {
            let caller = self.get_caller();
            let transferred_funds = self.check_and_get_transferred_funds()?;
            let account_balance = self.get_balance_by_account().unwrap_or(0);

            let new_balance = account_balance + transferred_funds;

            self.balances.insert(caller, &new_balance);

            self.env().emit_event(Deposited {
                from: caller,
                balance: transferred_funds,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn withdraw(&mut self, withdrawal_amount: Option<u128>) -> Result<(), ContractError> {
            let caller = self.get_caller();
            let mut account_balance = self.get_balance_by_account()?;

            if account_balance == 0 {
                return Err(ContractError::AccountWithoutBalance);
            }

            let withdrawal_amount = withdrawal_amount.unwrap_or(account_balance);

            if withdrawal_amount > account_balance {
                return Err(ContractError::ExpectedWithdrawalAmountExceedsAccountBalance);
            }

            account_balance -= withdrawal_amount;
            self.balances.insert(caller, &account_balance);

            if self.env().transfer(caller, withdrawal_amount).is_err() {
                return Err(ContractError::WithdrawTransferFailed);
            }

            self.env().emit_event(Withdrawn {
                to: caller,
                balance: withdrawal_amount,
            });

            Ok(())
        }

        fn get_caller(&self) -> AccountId {
            self.env().caller()
        }

        fn check_and_get_transferred_funds(&self) -> Result<u128, ContractError> {
            let transferred_funds = self.env().transferred_value();
            if transferred_funds == 0 {
                return Err(ContractError::InsufficientFunds);
            }

            Ok(transferred_funds)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink_lang as ink;

        fn get_default_accounts() -> ink_env::test::DefaultAccounts<ink_env::DefaultEnvironment> {
            ink_env::test::default_accounts::<ink_env::DefaultEnvironment>()
        }

        fn init() -> (
            Workshop,
            ink_env::test::DefaultAccounts<ink_env::DefaultEnvironment>,
        ) {
            (Workshop::new(), get_default_accounts())
        }

        fn set_caller(sender: AccountId) {
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(sender);
        }

        #[ink::test]
        fn withdraw_works() {
            // Arrange
            let (mut contract, accounts) = init();
            let caller = accounts.bob;
            let balance_amount = 1000;
            let withdrawal_amount = 600;
            contract.balances.insert(caller, &balance_amount);
            set_caller(caller);

            // Act
            contract.withdraw(Some(withdrawal_amount)).unwrap();
            let result = contract.balances.get(caller).unwrap();

            // Assert
            assert_eq!(result, balance_amount - withdrawal_amount);
        }

        #[ink::test]
        fn withdraw_fails() {
            // Arrange
            let (mut contract, accounts) = init();
            let caller = accounts.bob;
            set_caller(caller);

            // Act
            let result = contract.withdraw(None);

            // Assert
            assert_eq!(result, Err(ContractError::AccountWithoutBalance));
        }
    }
}
