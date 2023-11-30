use near_sdk::serde_json::json;
use near_workspaces::result::ExecutionFinalResult;
use near_workspaces::{Account, AccountId, Contract};

#[tokio::test]
async fn test_workflow() -> anyhow::Result<()> {
    // Compile the contract.
    let wasm = near_workspaces::compile_project(".").await?;

    // Spin up a sandbox and deploy the contract.
    let worker = near_workspaces::sandbox().await?;
    let contract = worker.dev_deploy(&wasm).await?;

    // Call the contract's constructor.
    // We attach `max_gas()` to contract calls to keep it simple. The amount of gas actually used is
    // not relevant in this example.
    contract
        .call("new")
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    // Verify the contract was set up correctly.
    assert_eq!(get_counter_value(&contract).await?, 0);

    // Create an account that will not be granted any ACL roles.
    let unauthorized_account = worker.dev_create_account().await?;

    // Verify `increment` can be called by anyone.
    call_counter_modifier(&contract, &unauthorized_account, "increment")
        .await?
        .into_result()?;
    assert_eq!(get_counter_value(&contract).await?, 1);

    // Verify `decrement` can _not_ be called by an unauthorized account.
    let res = call_counter_modifier(&contract, &unauthorized_account, "decrement").await?;
    assert_failure_with(
        res,
        "Insufficient permissions for method decrement restricted by access control",
    );

    // Create an account that will be granted the `Decrementer` role.
    let decrementer_account = worker.dev_create_account().await?;

    // Verify an unauthorized account can _not_ grant roles.
    let res = acl_grant_role(
        &contract,
        &unauthorized_account,
        "Decrementer",
        decrementer_account.id(),
    )
    .await?;
    assert_eq!(
        res, None,
        "Expecting `None` which signalizes the caller misses permission to grant the role"
    );

    // The contract itself is made super admin in the constructor, hence it may grant all roles.
    let res = acl_grant_role(
        &contract,
        &contract.as_account(),
        "Decrementer",
        decrementer_account.id(),
    )
    .await?;
    assert_eq!(
        res,
        Some(true),
        "Expecting `Some(true)` which signalizes the role was granted"
    );

    // Remember, the current counter value is 1.
    assert_eq!(get_counter_value(&contract).await?, 1);

    // After being granted the role, `decrementer_account` may successfully call `decrement`.
    call_counter_modifier(&contract, &decrementer_account, "decrement")
        .await?
        .into_result()?;
    assert_eq!(get_counter_value(&contract).await?, 0);

    // Above workflow provides guidance on how to interact with an `AccessControllable` contract.
    //
    // The `Resetter` role can be granted in the same manner.
    //
    // For a complete overview of the API implemented for an `AccessControllable` contract, you may
    // refer to the documentation of the `AccessControllable` trait.

    Ok(())
}

/// Returns the current value of the counter `contract`.
async fn get_counter_value(contract: &Contract) -> anyhow::Result<i64> {
    let value: i64 = contract.call("value").view().await?.json()?;
    Ok(value)
}

/// Uses `caller` to call one of the modifier methods (`increment`, `decrement`, `reset`) on the
/// counter `contract`.
async fn call_counter_modifier(
    contract: &Contract,
    caller: &Account,
    method_name: &str,
) -> near_workspaces::Result<ExecutionFinalResult, near_workspaces::error::Error> {
    caller
        .call(contract.id(), method_name)
        .max_gas()
        .transact()
        .await
}

/// Calls the `acl_grant_role` method on `contract`` via `caller`. Parameters `role` and
/// `account_id` are passed on to `acl_grant_role`.
///
/// Assumes `contract` is `AccessControllable`, which provides an automatic implentation of
/// `acl_grant_role`. See the `AccessControllable` trait for details of `acl_grant_role`.
async fn acl_grant_role(
    contract: &Contract,
    caller: &Account,
    role: &str,
    account_id: &AccountId,
) -> anyhow::Result<Option<bool>> {
    let res = caller
        .call(contract.id(), "acl_grant_role")
        .args_json(json!({
            "role": role,
            "account_id": account_id,
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?
        .json::<Option<bool>>()?;
    Ok(res)
}

/// Asserts the execution of `res` failed and the error contains the provided message.
pub fn assert_failure_with(res: ExecutionFinalResult, must_contain: &str) {
    let err = res
        .into_result()
        .expect_err("Transaction should have failed");
    let err = format!("{}", err);
    assert!(
        err.contains(must_contain),
        "The expected message\n'{}'\nis not contained in error\n'{}'",
        must_contain,
        err,
    );
}
