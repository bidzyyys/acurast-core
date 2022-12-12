#[cfg(feature = "attestation")]
mod bounded_attestation;

#[cfg(feature = "attestation")]
pub use bounded_attestation::*;

use frame_support::{
    pallet_prelude::*, sp_runtime::traits::MaybeDisplay, storage::bounded_vec::BoundedVec,
};
use sp_std::prelude::*;

pub(crate) const SCRIPT_PREFIX: &[u8] = b"ipfs://";
pub(crate) const SCRIPT_LENGTH: u32 = 53;

/// Type representing the utf8 bytes of a string containing the value of an ipfs url.
/// The ipfs url is expected to point to a script.
pub type Script = BoundedVec<u8, ConstU32<SCRIPT_LENGTH>>;

pub fn is_valid_script(script: &Script) -> bool {
    let script_len: u32 = script.len().try_into().unwrap_or(0);
    script_len == SCRIPT_LENGTH && script.starts_with(SCRIPT_PREFIX)
}

/// https://datatracker.ietf.org/doc/html/rfc5280#section-4.1.2.2
const SERIAL_NUMBER_MAX_LENGTH: u32 = 20;

pub type SerialNumber = BoundedVec<u8, ConstU32<SERIAL_NUMBER_MAX_LENGTH>>;

/// Structure representing a job fulfillment. It contains the script that generated the payload and the actual payload.
#[derive(RuntimeDebug, Encode, Decode, TypeInfo, Clone, PartialEq)]
pub struct Fulfillment {
    /// The script that generated the payload.
    pub script: Script,
    /// The output of a script.
    pub payload: Vec<u8>,
}

/// Structure used to updated the allowed sources list of a [Registration].
#[derive(RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq)]
pub struct AllowedSourcesUpdate<A>
where
    A: Parameter + Member + MaybeSerializeDeserialize + MaybeDisplay + Ord + MaxEncodedLen,
{
    /// The update operation.
    pub operation: ListUpdateOperation,
    /// The [AccountId] to add or remove.
    pub account_id: A,
}

/// A Job ID consists of an [AccountId] and a [Script].
pub type JobId<AccountId> = (AccountId, Script);

/// Structure used to updated the allowed sources list of a [Registration].
#[derive(RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq)]
pub struct JobAssignmentUpdate<A>
where
    A: Parameter + Member + MaybeSerializeDeserialize + MaybeDisplay + Ord + MaxEncodedLen,
{
    /// The update operation.
    pub operation: ListUpdateOperation,
    /// The [AccountId] to assign the job to.
    pub assignee: A,
    /// the job id to be assigned.
    pub job_id: JobId<A>,
}

/// Structure used to updated the certificate recovation list.
#[derive(RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq)]
pub struct CertificateRevocationListUpdate {
    /// The update operation.
    pub operation: ListUpdateOperation,
    /// The [AccountId] to add or remove.
    pub cert_serial_number: SerialNumber,
}

/// The allowed sources update operation.
#[derive(RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq, Copy)]
pub enum ListUpdateOperation {
    Add,
    Remove,
}

/// Structure representing a job registration.
#[derive(RuntimeDebug, Encode, Decode, TypeInfo, Clone, PartialEq)]
pub struct JobRegistration<AccountId, Extra>
where
    AccountId: Parameter + Member + MaybeSerializeDeserialize + MaybeDisplay + Ord,
    Extra: Parameter + Member,
{
    /// The script to execute. It is a vector of bytes representing a utf8 string. The string needs to be a ipfs url that points to the script.
    pub script: Script,
    /// An optional array of the [AccountId]s allowed to fulfill the job. If the array is [None], then all sources are allowed.
    pub allowed_sources: Option<Vec<AccountId>>,
    /// A boolean indicating if only verified sources can fulfill the job. A verified source is one that has provided a valid key attestation.
    pub allow_only_verified_sources: bool,
    /// Extra parameters. This type can be configured through [Config::RegistrationExtra].
    pub extra: Extra,
}

/// Structure representing a job registration.
#[derive(RuntimeDebug, Encode, Decode, TypeInfo, Clone, Eq, PartialEq)]
pub struct JobRequirements<Reward>
where
    Reward: Parameter + Member,
{
    /// The number of execution slots to be assigned to distinct sources. Either all or no slot get assigned by matching.
    pub slots: u8,
    /// CPU milliseconds (upper bound) required to execute script.
    pub cpu_milliseconds: u128,
    /// Reward offered for the job
    pub reward: Reward,
}

/// Calls a default value that makes a register extrinsic called by "consumer" pass when
/// built with "runtime-benchmarks" feature. Depending on your implementation, you might need a
/// specific genesis config to achieve this.
/// For example if your logic requires a minted asset to be specified, your genesis config should
/// have the consumer hold enough balance of the token specified in the return struct of this fn
/// ( as well as an existential deposit for "consumer" )
pub trait BenchmarkDefault {
    fn benchmark_default() -> Self;
}

impl BenchmarkDefault for () {
    fn benchmark_default() -> Self {
        ()
    }
}

pub trait BenchmarkDefaultValue<T> {
    fn default() -> T;
}
