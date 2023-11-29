use near_plugins::{access_control, access_control_any, AccessControlRole, AccessControllable};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, near_bindgen, PanicOnDefault};

/// Roles are represented by enum variants.
///
/// Deriving `AccessControlRole` ensures `Role` can be used in `AccessControllable`.
#[derive(AccessControlRole, Deserialize, Serialize, Copy, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum Role {
    /// Grantees of this role may decrease the counter.
    Decrementer,
    /// Grantees of this role may reset the counter.
    Resetter,
}

/// Pass `Role` to the `access_controllable` macro.
#[access_control(role_type(Role))]
#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct Counter {
    value: i64,
}

#[near_bindgen]
impl Counter {
    #[init]
    pub fn new() -> Self {
        let mut contract = Self { value: 0 };

        // Make the contract itself super admin.
        near_sdk::require!(
            contract.acl_init_super_admin(env::current_account_id()),
            "Failed to initialize super admin",
        );

        contract
    }

    /// Anyone can retrieve the current value.
    pub fn value(&self) -> i64 {
        self.value
    }

    /// Increases the value of the counter by one.
    ///
    /// Anyone can increase the counter.
    pub fn increment(&mut self) {
        self.value += 1;
    }

    /// Decreases the value of the counter by one.
    ///
    /// Only accounts that have been granted `Role::Decrementer` may successfully call this method.
    /// If called by an account without this role, the method panics and state remains unchanged.
    #[access_control_any(roles(Role::Decrementer))] // enables ACL for this method
    pub fn decrement(&mut self) {
        self.value -= 1;
    }

    /// Resets the value of the counter to zero.
    ///
    /// Only accounts that have been granted `Role::Resetter` may successfully call this method.
    /// If called by an account without this role, the method panics and state remains unchanged.
    #[access_control_any(roles(Role::Resetter))] // enables ACL for this method
    pub fn reset(&mut self) {
        self.value = 0;
    }

    /// By the way, it is also possible to restrict access to accounts that have been granted any of
    /// multiple roles. This is how the syntax looks.
    #[access_control_any(roles(Role::Decrementer, Role::Resetter))]
    pub fn no_op(&self) {}
}
