use soroban_sdk::contracterror;

/// All error conditions the carbon registry can return.
///
/// Using a typed error enum (instead of `panic!`) means callers and the
/// frontend get a stable, numeric error code they can branch on, and the
/// contract never aborts with an opaque host trap for expected failures.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// `initialize` was called more than once.
    AlreadyInitialized = 1,
    /// A read expected the contract to be initialized but it was not.
    NotInitialized = 2,
    /// Referenced project id does not exist.
    ProjectNotFound = 3,
    /// Referenced listing id does not exist (or was fully sold / cancelled).
    ListingNotFound = 4,
    /// Referenced retirement certificate id does not exist.
    RetirementNotFound = 5,
    /// A numeric amount argument was zero where a positive value is required.
    InvalidAmount = 6,
    /// The project does not have enough un-listed credits available.
    InsufficientAvailable = 7,
    /// The caller does not hold enough credits for this operation.
    InsufficientBalance = 8,
    /// The listing does not have enough remaining credits for this purchase.
    InsufficientListed = 9,
    /// Arithmetic overflowed a u128 accumulator.
    Overflow = 10,
    /// Caller is not the configured admin.
    NotAuthorizedAdmin = 11,
    /// Caller is not the configured verifier.
    NotAuthorizedVerifier = 12,
    /// Caller is not the owner/seller of the referenced entity.
    NotAuthorizedSeller = 13,
    /// A metadata string exceeded the maximum allowed length.
    MetadataTooLong = 14,
}
