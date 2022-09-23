use frame_support::{
    pallet_prelude::*,
    sp_runtime::traits::{MaybeDisplay, StaticLookup},
    storage::bounded_vec::BoundedVec,
};
use sp_std::prelude::*;

use crate::attestation::{
    asn::{self, KeyDescription},
    CertificateChainInput, CHAIN_MAX_LENGTH,
};

use crate::Config;
use sp_std::prelude::*;
use xcm::latest::prelude::*;

pub const MAX_PRICING_VARIANTS: u32 = 100;

/// The resource advertisement by a source containing pricing and capacity announcements.
#[derive(RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq)]
pub struct Advertisement<C>
where
    C: Parameter + Member + MaybeSerializeDeserialize + MaybeDisplay + Ord + MaxEncodedLen,
{
    /// The reward token accepted. Understood as one-of per job assigned.
    pub pricing: BoundedVec<PricingVariant, ConstU32<MAX_PRICING_VARIANTS>>,
    // Capacity not too be exceeded in matching.
    pub capacity: u32,
    /// An optional array of the [AccountId]s of consumers whose jobs should get accepted. If the array is [None], then jobs from all consumers are accepted.
    pub allowed_consumers: Option<Vec<C>>,
}

/// A type to dynamically describe an asset used for rewarding.
/// The same asset is also used for slashing.
/// TODO maybe this can be generalized to AssetId
#[derive(RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq)]
pub struct RewardAssetId(pub MultiLocation);

type MultiAssetIdExtractionError = ();
type MultiAssetValueExtractionError = ();

pub fn extract_asset(
    multi_asset: MultiAsset,
) -> Result<RewardAssetId, MultiAssetIdExtractionError> {
    match multi_asset.id {
        Concrete(multi_location) => Ok(RewardAssetId(multi_location)),
        _ => Err(()),
    }
}

pub fn extract_value(multi_asset: &MultiAsset) -> Result<u128, MultiAssetValueExtractionError> {
    match multi_asset.fun {
        Fungible(amount) => Ok(amount),
        NonFungible(_) => Err(()),
    }
}

impl RewardAssetId {
    pub fn with_value(&self, value: u128) -> MultiAsset {
        MultiAsset {
            id: Concrete(self.0.clone()),
            fun: Fungible(value),
        }
    }
}

pub type Price = u128;

/// Pricing variant listing cost per resource unit and slash on SLA violation.
/// Specified in specific asset that is payed out or deducted from stake on complete fulfillment.
#[derive(RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq)]
pub struct PricingVariant {
    /// The rewarded asset. Only one per [PricingVariant].
    pub reward_asset: RewardAssetId,
    /// Price in [reward_asset] per cpu second.
    pub price_per_cpu_millisecond: Price, // TODO change to nanosecond
    /// A fixed bonus in [reward_asset].
    pub bonus: u128,
    /// The maximum slash to put at stake and that is lost if SLA is violated.
    pub maximum_slash: u128,
}

pub type AdvertismentIndexValue<AccountId> = (AccountId, Price);

pub(crate) const SCRIPT_PREFIX: &[u8] = b"ipfs://";
pub(crate) const SCRIPT_LENGTH: u32 = 53;

/// Type representing the utf8 bytes of a string containing the value of an ipfs url.
/// The ipfs url is expected to point to a script.
pub type Script = BoundedVec<u8, ConstU32<SCRIPT_LENGTH>>;

/// https://datatracker.ietf.org/doc/html/rfc5280#section-4.1.2.2
const ISSUER_NAME_MAX_LENGTH: u32 = 64;
const SERIAL_NUMBER_MAX_LENGTH: u32 = 20;

pub type IssuerName = BoundedVec<u8, ConstU32<ISSUER_NAME_MAX_LENGTH>>;
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
    pub operation: JobAssignemntUpdateOperation,
    /// The [AccountId] to assign the job to.
    pub assignee: A,
    /// The job's original  to be assigned.
    pub requester: A,
    /// The consumer that registered the job originally.
    pub script: Script,
}

/// The allowed sources update operation.
#[derive(RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq, Copy)]
pub enum JobAssignemntUpdateOperation {
    Add(u8), // The slot to assign.
    Remove,
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

/// The allowed sources update operation.
#[derive(RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq, Copy)]
pub enum JobStatus {
    Open,
    Assigned,
    Fulfilled(SLAEvaluation),
}

impl Default for JobStatus {
    fn default() -> Self {
        JobStatus::Open
    }
}

/// Represents an evaluation of the SLA after a job's schedule is completed.
#[derive(RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq, Copy)]
pub struct SLAEvaluation {
    total: u8,
    met: u8,
}

/// Structure representing a job with its registration and curent status.
#[derive(RuntimeDebug, Encode, Decode, TypeInfo, Clone, PartialEq)]
pub struct Job<A, T>
where
    A: Parameter + Member + MaybeSerializeDeserialize + MaybeDisplay + Ord + MaxEncodedLen,
    T: Parameter + Member + MaxEncodedLen,
{
    pub registration: JobRegistration<A, T>,
    pub status: JobStatus,
}

/// Structure representing a job registration.
#[derive(RuntimeDebug, Encode, Decode, TypeInfo, Clone, PartialEq)]
pub struct JobRegistration<A, T>
where
    A: Parameter + Member + MaybeSerializeDeserialize + MaybeDisplay + Ord + MaxEncodedLen,
    T: Parameter + Member + MaxEncodedLen,
{
    /// The script to execute. It is a vector of bytes representing a utf8 string. The string needs to be a ipfs url that points to the script.
    pub script: Script,
    /// The number of execution slots to be assigned to distinct sources. Either all or no slot get assigned by matching.
    pub slots: u8,
    /// CPU milliseconds (upper bound) required to execute script.
    pub cpu_milliseconds: u128,
    /// An optional array of the [AccountId]s allowed to fulfill the job. If the array is [None], then all sources are allowed.
    pub allowed_sources: Option<Vec<A>>,
    /// A boolean indicating if only verified sources can fulfill the job. A verified source is one that has provided a valid key attestation.
    pub allow_only_verified_sources: bool,
    /// Total reward (and reward type) offered for the job.
    pub reward: xcm::v2::MultiAsset,
    /// Extra parameters. This type can be configured through [Config::RegistrationExtra].
    pub extra: T,
}

pub(crate) const PURPOSE_MAX_LENGTH: u32 = 50;
pub(crate) const DIGEST_MAX_LENGTH: u32 = 32;
pub(crate) const PADDING_MAX_LENGTH: u32 = 32;
pub(crate) const MGF_DIGEST_MAX_LENGTH: u32 = 32;
pub(crate) const VERIFIED_BOOT_KEY_MAX_LENGTH: u32 = 32;
pub(crate) const VERIFIED_BOOT_HASH_MAX_LENGTH: u32 = 32;
pub(crate) const ATTESTATION_ID_MAX_LENGTH: u32 = 256;
pub(crate) const BOUDNED_SET_PROPERTY: u32 = 16;

pub type Purpose = BoundedVec<u8, ConstU32<PURPOSE_MAX_LENGTH>>;
pub type Digest = BoundedVec<u8, ConstU32<DIGEST_MAX_LENGTH>>;
pub type Padding = BoundedVec<u8, ConstU32<PADDING_MAX_LENGTH>>;
pub type MgfDigest = BoundedVec<u8, ConstU32<MGF_DIGEST_MAX_LENGTH>>;
pub type VerifiedBootKey = BoundedVec<u8, ConstU32<VERIFIED_BOOT_KEY_MAX_LENGTH>>;
pub type VerifiedBootHash = BoundedVec<u8, ConstU32<VERIFIED_BOOT_HASH_MAX_LENGTH>>;
pub type AttestationIdProperty = BoundedVec<u8, ConstU32<ATTESTATION_ID_MAX_LENGTH>>;
pub type CertId = (IssuerName, SerialNumber);
pub type ValidatingCertIds = BoundedVec<CertId, ConstU32<CHAIN_MAX_LENGTH>>;
pub type BoundedSetProperty = BoundedVec<CertId, ConstU32<BOUDNED_SET_PROPERTY>>;

/// Structure representing a submitted attestation chain.
#[derive(RuntimeDebug, Encode, Decode, TypeInfo, Clone, PartialEq)]
pub struct AttestationChain {
    /// An ordered array of [CertificateInput]s describing a valid chain from known root certificate to attestation certificate.
    pub certificate_chain: CertificateChainInput,
}

/// Structure representing a stored attestation.
#[derive(RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq)]
pub struct Attestation {
    pub cert_ids: ValidatingCertIds,
    pub key_description: BoundedKeyDescription,
    pub validity: AttestationValidity,
}

#[derive(RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo, Clone, Copy, PartialEq, Eq)]
pub struct AttestationValidity {
    pub not_before: u64,
    pub not_after: u64,
}

#[derive(RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq)]
pub struct BoundedKeyDescription {
    pub attestation_security_level: AttestationSecurityLevel,
    pub key_mint_security_level: AttestationSecurityLevel,
    pub software_enforced: BoundedAuthorizationList,
    pub tee_enforced: BoundedAuthorizationList,
}

impl TryFrom<KeyDescription<'_>> for BoundedKeyDescription {
    type Error = ();

    fn try_from(value: KeyDescription) -> Result<Self, Self::Error> {
        match value {
            KeyDescription::V1(kd) => kd.try_into(),
            KeyDescription::V2(kd) => kd.try_into(),
            KeyDescription::V3(kd) => kd.try_into(),
            KeyDescription::V4(kd) => kd.try_into(),
            KeyDescription::V100(kd) => kd.try_into(),
            KeyDescription::V200(kd) => kd.try_into(),
        }
    }
}

impl TryFrom<asn::KeyDescriptionV1<'_>> for BoundedKeyDescription {
    type Error = ();

    fn try_from(data: asn::KeyDescriptionV1) -> Result<Self, Self::Error> {
        Ok(BoundedKeyDescription {
            attestation_security_level: data.attestation_security_level.into(),
            key_mint_security_level: data.key_mint_security_level.into(),
            software_enforced: data.software_enforced.try_into()?,
            tee_enforced: data.tee_enforced.try_into()?,
        })
    }
}

impl TryFrom<asn::KeyDescriptionV2<'_>> for BoundedKeyDescription {
    type Error = ();

    fn try_from(data: asn::KeyDescriptionV2) -> Result<Self, Self::Error> {
        Ok(BoundedKeyDescription {
            attestation_security_level: data.attestation_security_level.into(),
            key_mint_security_level: data.key_mint_security_level.into(),
            software_enforced: data.software_enforced.try_into()?,
            tee_enforced: data.tee_enforced.try_into()?,
        })
    }
}

impl TryFrom<asn::KeyDescriptionV3<'_>> for BoundedKeyDescription {
    type Error = ();

    fn try_from(data: asn::KeyDescriptionV3) -> Result<Self, Self::Error> {
        Ok(BoundedKeyDescription {
            attestation_security_level: data.attestation_security_level.into(),
            key_mint_security_level: data.key_mint_security_level.into(),
            software_enforced: data.software_enforced.try_into()?,
            tee_enforced: data.tee_enforced.try_into()?,
        })
    }
}

impl TryFrom<asn::KeyDescriptionV4<'_>> for BoundedKeyDescription {
    type Error = ();

    fn try_from(data: asn::KeyDescriptionV4) -> Result<Self, Self::Error> {
        Ok(BoundedKeyDescription {
            attestation_security_level: data.attestation_security_level.into(),
            key_mint_security_level: data.key_mint_security_level.into(),
            software_enforced: data.software_enforced.try_into()?,
            tee_enforced: data.tee_enforced.try_into()?,
        })
    }
}

impl TryFrom<asn::KeyDescriptionV100V200<'_>> for BoundedKeyDescription {
    type Error = ();

    fn try_from(data: asn::KeyDescriptionV100V200) -> Result<Self, Self::Error> {
        Ok(BoundedKeyDescription {
            attestation_security_level: data.attestation_security_level.into(),
            key_mint_security_level: data.key_mint_security_level.into(),
            software_enforced: data.software_enforced.try_into()?,
            tee_enforced: data.tee_enforced.try_into()?,
        })
    }
}

#[derive(RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq, Eq)]
pub enum AttestationSecurityLevel {
    Software,
    TrustedEnvironemnt,
    StrongBox,
    Unknown,
}

impl From<asn::SecurityLevel> for AttestationSecurityLevel {
    fn from(data: asn::SecurityLevel) -> Self {
        match data.value() {
            0 => AttestationSecurityLevel::Software,
            1 => AttestationSecurityLevel::TrustedEnvironemnt,
            2 => AttestationSecurityLevel::StrongBox,
            _ => AttestationSecurityLevel::Unknown,
        }
    }
}

#[derive(RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq)]
pub struct BoundedAuthorizationList {
    pub purpose: Option<Purpose>,
    pub algorithm: Option<u8>,
    pub key_size: Option<u16>,
    pub digest: Option<Digest>,
    pub padding: Option<Padding>,
    pub ec_curve: Option<u8>,
    pub rsa_public_exponent: Option<u64>,
    pub mgf_digest: Option<MgfDigest>,
    pub rollback_resistance: Option<bool>,
    pub early_boot_only: Option<bool>,
    pub active_date_time: Option<u64>,
    pub origination_expire_date_time: Option<u64>,
    pub usage_expire_date_time: Option<u64>,
    pub usage_count_limit: Option<u64>,
    pub no_auth_required: bool,
    pub user_auth_type: Option<u8>,
    pub auth_timeout: Option<u32>,
    pub allow_while_on_body: bool,
    pub trusted_user_presence_required: Option<bool>,
    pub trusted_confirmation_required: Option<bool>,
    pub unlocked_device_required: Option<bool>,
    pub all_applications: Option<bool>,
    pub application_id: Option<AttestationIdProperty>,
    pub creation_date_time: Option<u64>,
    pub origin: Option<u8>,
    pub root_of_trust: Option<BoundedRootOfTrust>,
    pub os_version: Option<u32>,
    pub os_patch_level: Option<u32>,
    pub attestation_application_id: Option<AttestationIdProperty>,
    pub attestation_id_brand: Option<AttestationIdProperty>,
    pub attestation_id_device: Option<AttestationIdProperty>,
    pub attestation_id_product: Option<AttestationIdProperty>,
    pub attestation_id_serial: Option<AttestationIdProperty>,
    pub attestation_id_imei: Option<AttestationIdProperty>,
    pub attestation_id_meid: Option<AttestationIdProperty>,
    pub attestation_id_manufacturer: Option<AttestationIdProperty>,
    pub attestation_id_model: Option<AttestationIdProperty>,
    pub vendor_patch_level: Option<u32>,
    pub boot_patch_level: Option<u32>,
    pub device_unique_attestation: Option<bool>,
}

macro_rules! try_bound_set {
    ( $set:expr, $target_vec_type:ty, $target_type:ty ) => {{
        $set.map(|v| {
            v.map(|i| <$target_type>::try_from(i))
                .collect::<Result<Vec<$target_type>, _>>()
        })
        .map_or(Ok(None), |r| r.map(Some))
        .map_err(|_| ())?
        .map(|v| <$target_vec_type>::try_from(v))
        .map_or(Ok(None), |r| r.map(Some))
    }};
}

macro_rules! try_bound {
    ( $v:expr, $target_type:ty ) => {{
        $v.map(|v| <$target_type>::try_from(v))
            .map_or(Ok(None), |r| r.map(Some))
            .map_err(|_| ())
    }};
}

/// The Authorization List tags. [Tag descriptions](https://source.android.com/docs/security/keystore/tags)
impl TryFrom<asn::AuthorizationListV1<'_>> for BoundedAuthorizationList {
    type Error = ();

    fn try_from(data: asn::AuthorizationListV1) -> Result<Self, Self::Error> {
        Ok(BoundedAuthorizationList {
            purpose: try_bound_set!(data.purpose, Purpose, u8)?,
            algorithm: try_bound!(data.algorithm, u8)?,
            key_size: try_bound!(data.key_size, u16)?,
            digest: try_bound_set!(data.digest, Digest, u8)?,
            padding: try_bound_set!(data.padding, Padding, u8)?,
            ec_curve: try_bound!(data.ec_curve, u8)?,
            rsa_public_exponent: try_bound!(data.rsa_public_exponent, u64)?,
            mgf_digest: None,
            rollback_resistance: Some(data.rollback_resistance.is_some()),
            early_boot_only: None,
            active_date_time: try_bound!(data.active_date_time, u64)?,
            origination_expire_date_time: try_bound!(data.origination_expire_date_time, u64)?,
            usage_expire_date_time: try_bound!(data.usage_expire_date_time, u64)?,
            usage_count_limit: None,
            no_auth_required: data.no_auth_required.is_some(),
            user_auth_type: try_bound!(data.user_auth_type, u8)?,
            auth_timeout: try_bound!(data.user_auth_type, u32)?,
            allow_while_on_body: data.allow_while_on_body.is_some(),
            trusted_user_presence_required: None,
            trusted_confirmation_required: None,
            unlocked_device_required: None,
            all_applications: Some(data.all_applications.is_some()),
            application_id: data
                .application_id
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            creation_date_time: try_bound!(data.creation_date_time, u64)?,
            origin: try_bound!(data.origin, u8)?,
            root_of_trust: data
                .root_of_trust
                .map(|v| v.try_into())
                .map_or(Ok(None), |r| r.map(Some))?,
            os_version: try_bound!(data.os_version, u32)?,
            os_patch_level: try_bound!(data.os_patch_level, u32)?,
            vendor_patch_level: None,
            attestation_application_id: None,
            attestation_id_brand: None,
            attestation_id_device: None,
            attestation_id_product: None,
            attestation_id_serial: None,
            attestation_id_imei: None,
            attestation_id_meid: None,
            attestation_id_manufacturer: None,
            attestation_id_model: None,
            boot_patch_level: None,
            device_unique_attestation: None,
        })
    }
}

impl TryFrom<asn::AuthorizationListV2<'_>> for BoundedAuthorizationList {
    type Error = ();

    fn try_from(data: asn::AuthorizationListV2) -> Result<Self, Self::Error> {
        Ok(BoundedAuthorizationList {
            purpose: try_bound_set!(data.purpose, Purpose, u8)?,
            algorithm: try_bound!(data.algorithm, u8)?,
            key_size: try_bound!(data.key_size, u16)?,
            digest: try_bound_set!(data.digest, Digest, u8)?,
            padding: try_bound_set!(data.padding, Padding, u8)?,
            ec_curve: try_bound!(data.ec_curve, u8)?,
            rsa_public_exponent: try_bound!(data.rsa_public_exponent, u64)?,
            mgf_digest: None,
            rollback_resistance: Some(data.rollback_resistance.is_some()),
            early_boot_only: None,
            active_date_time: try_bound!(data.active_date_time, u64)?,
            origination_expire_date_time: try_bound!(data.origination_expire_date_time, u64)?,
            usage_expire_date_time: try_bound!(data.usage_expire_date_time, u64)?,
            usage_count_limit: None,
            no_auth_required: data.no_auth_required.is_some(),
            user_auth_type: try_bound!(data.user_auth_type, u8)?,
            auth_timeout: try_bound!(data.user_auth_type, u32)?,
            allow_while_on_body: data.allow_while_on_body.is_some(),
            trusted_user_presence_required: None,
            trusted_confirmation_required: None,
            unlocked_device_required: None,
            all_applications: Some(data.all_applications.is_some()),
            application_id: data
                .application_id
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            creation_date_time: try_bound!(data.creation_date_time, u64)?,
            origin: try_bound!(data.origin, u8)?,
            root_of_trust: data
                .root_of_trust
                .map(|v| v.try_into())
                .map_or(Ok(None), |r| r.map(Some))?,
            os_version: try_bound!(data.os_version, u32)?,
            os_patch_level: try_bound!(data.os_patch_level, u32)?,
            attestation_application_id: data
                .attestation_application_id
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_brand: data
                .attestation_id_brand
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_device: data
                .attestation_id_device
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_product: data
                .attestation_id_product
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_serial: data
                .attestation_id_serial
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_imei: data
                .attestation_id_imei
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_meid: data
                .attestation_id_meid
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_manufacturer: data
                .attestation_id_manufacturer
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_model: data
                .attestation_id_model
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            vendor_patch_level: None,
            boot_patch_level: None,
            device_unique_attestation: None,
        })
    }
}

impl TryFrom<asn::AuthorizationListV3<'_>> for BoundedAuthorizationList {
    type Error = ();

    fn try_from(data: asn::AuthorizationListV3) -> Result<Self, Self::Error> {
        Ok(BoundedAuthorizationList {
            purpose: try_bound_set!(data.purpose, Purpose, u8)?,
            algorithm: try_bound!(data.algorithm, u8)?,
            key_size: try_bound!(data.key_size, u16)?,
            digest: try_bound_set!(data.digest, Digest, u8)?,
            padding: try_bound_set!(data.padding, Padding, u8)?,
            ec_curve: try_bound!(data.ec_curve, u8)?,
            rsa_public_exponent: try_bound!(data.rsa_public_exponent, u64)?,
            mgf_digest: None,
            rollback_resistance: Some(data.rollback_resistance.is_some()),
            early_boot_only: None,
            active_date_time: try_bound!(data.active_date_time, u64)?,
            origination_expire_date_time: try_bound!(data.origination_expire_date_time, u64)?,
            usage_expire_date_time: try_bound!(data.usage_expire_date_time, u64)?,
            usage_count_limit: None,
            no_auth_required: data.no_auth_required.is_some(),
            user_auth_type: try_bound!(data.user_auth_type, u8)?,
            auth_timeout: try_bound!(data.user_auth_type, u32)?,
            allow_while_on_body: data.allow_while_on_body.is_some(),
            trusted_user_presence_required: None,
            trusted_confirmation_required: None,
            unlocked_device_required: None,
            all_applications: Some(data.all_applications.is_some()),
            application_id: data
                .application_id
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            creation_date_time: try_bound!(data.creation_date_time, u64)?,
            origin: try_bound!(data.origin, u8)?,
            root_of_trust: data
                .root_of_trust
                .map(|v| v.try_into())
                .map_or(Ok(None), |r| r.map(Some))?,
            os_version: try_bound!(data.os_version, u32)?,
            os_patch_level: try_bound!(data.os_patch_level, u32)?,
            attestation_application_id: data
                .attestation_application_id
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_brand: data
                .attestation_id_brand
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_device: data
                .attestation_id_device
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_product: data
                .attestation_id_product
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_serial: data
                .attestation_id_serial
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_imei: data
                .attestation_id_imei
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_meid: data
                .attestation_id_meid
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_manufacturer: data
                .attestation_id_manufacturer
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_model: data
                .attestation_id_model
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            vendor_patch_level: try_bound!(data.vendor_patch_level, u32)?,
            boot_patch_level: try_bound!(data.boot_patch_level, u32)?,
            device_unique_attestation: None,
        })
    }
}

impl TryFrom<asn::AuthorizationListV4<'_>> for BoundedAuthorizationList {
    type Error = ();

    fn try_from(data: asn::AuthorizationListV4) -> Result<Self, Self::Error> {
        Ok(BoundedAuthorizationList {
            purpose: try_bound_set!(data.purpose, Purpose, u8)?,
            algorithm: try_bound!(data.algorithm, u8)?,
            key_size: try_bound!(data.key_size, u16)?,
            digest: try_bound_set!(data.digest, Digest, u8)?,
            padding: try_bound_set!(data.padding, Padding, u8)?,
            ec_curve: try_bound!(data.ec_curve, u8)?,
            rsa_public_exponent: try_bound!(data.rsa_public_exponent, u64)?,
            mgf_digest: None,
            rollback_resistance: Some(data.rollback_resistance.is_some()),
            early_boot_only: Some(data.early_boot_only.is_some()),
            active_date_time: try_bound!(data.active_date_time, u64)?,
            origination_expire_date_time: try_bound!(data.origination_expire_date_time, u64)?,
            usage_expire_date_time: try_bound!(data.usage_expire_date_time, u64)?,
            usage_count_limit: None,
            no_auth_required: data.no_auth_required.is_some(),
            user_auth_type: try_bound!(data.user_auth_type, u8)?,
            auth_timeout: try_bound!(data.user_auth_type, u32)?,
            allow_while_on_body: data.allow_while_on_body.is_some(),
            trusted_user_presence_required: Some(data.trusted_user_presence_required.is_some()),
            trusted_confirmation_required: Some(data.trusted_confirmation_required.is_some()),
            unlocked_device_required: Some(data.unlocked_device_required.is_some()),
            all_applications: Some(data.all_applications.is_some()),
            application_id: data
                .application_id
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            creation_date_time: try_bound!(data.creation_date_time, u64)?,
            origin: try_bound!(data.origin, u8)?,
            root_of_trust: data
                .root_of_trust
                .map(|v| v.try_into())
                .map_or(Ok(None), |r| r.map(Some))?,
            os_version: try_bound!(data.os_version, u32)?,
            os_patch_level: try_bound!(data.os_patch_level, u32)?,
            attestation_application_id: data
                .attestation_application_id
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_brand: data
                .attestation_id_brand
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_device: data
                .attestation_id_device
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_product: data
                .attestation_id_product
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_serial: data
                .attestation_id_serial
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_imei: data
                .attestation_id_imei
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_meid: data
                .attestation_id_meid
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_manufacturer: data
                .attestation_id_manufacturer
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_model: data
                .attestation_id_model
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            vendor_patch_level: try_bound!(data.vendor_patch_level, u32)?,
            boot_patch_level: try_bound!(data.boot_patch_level, u32)?,
            device_unique_attestation: Some(data.device_unique_attestation.is_some()),
        })
    }
}

impl TryFrom<asn::AuthorizationListV100V200<'_>> for BoundedAuthorizationList {
    type Error = ();

    fn try_from(data: asn::AuthorizationListV100V200) -> Result<Self, Self::Error> {
        Ok(BoundedAuthorizationList {
            purpose: try_bound_set!(data.purpose, Purpose, u8)?,
            algorithm: try_bound!(data.algorithm, u8)?,
            key_size: try_bound!(data.key_size, u16)?,
            digest: try_bound_set!(data.digest, Digest, u8)?,
            padding: try_bound_set!(data.padding, Padding, u8)?,
            ec_curve: try_bound!(data.ec_curve, u8)?,
            rsa_public_exponent: try_bound!(data.rsa_public_exponent, u64)?,
            mgf_digest: try_bound_set!(data.mgf_digest, MgfDigest, u8)?,
            rollback_resistance: Some(data.rollback_resistance.is_some()),
            early_boot_only: Some(data.early_boot_only.is_some()),
            active_date_time: try_bound!(data.active_date_time, u64)?,
            origination_expire_date_time: try_bound!(data.origination_expire_date_time, u64)?,
            usage_expire_date_time: try_bound!(data.usage_expire_date_time, u64)?,
            usage_count_limit: try_bound!(data.usage_count_limit, u64)?,
            no_auth_required: data.no_auth_required.is_some(),
            user_auth_type: try_bound!(data.user_auth_type, u8)?,
            auth_timeout: try_bound!(data.user_auth_type, u32)?,
            allow_while_on_body: data.allow_while_on_body.is_some(),
            trusted_user_presence_required: Some(data.trusted_user_presence_required.is_some()),
            trusted_confirmation_required: Some(data.trusted_confirmation_required.is_some()),
            unlocked_device_required: Some(data.unlocked_device_required.is_some()),
            all_applications: None,
            application_id: None,
            creation_date_time: try_bound!(data.creation_date_time, u64)?,
            origin: try_bound!(data.origin, u8)?,
            root_of_trust: data
                .root_of_trust
                .map(|v| v.try_into())
                .map_or(Ok(None), |r| r.map(Some))?,
            os_version: try_bound!(data.os_version, u32)?,
            os_patch_level: try_bound!(data.os_patch_level, u32)?,
            attestation_application_id: data
                .attestation_application_id
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_brand: data
                .attestation_id_brand
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_device: data
                .attestation_id_device
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_product: data
                .attestation_id_product
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_serial: data
                .attestation_id_serial
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_imei: data
                .attestation_id_imei
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_meid: data
                .attestation_id_meid
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_manufacturer: data
                .attestation_id_manufacturer
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            attestation_id_model: data
                .attestation_id_model
                .map(|v| AttestationIdProperty::try_from(v.to_vec()))
                .map_or(Ok(None), |r| r.map(Some))?,
            vendor_patch_level: try_bound!(data.vendor_patch_level, u32)?,
            boot_patch_level: try_bound!(data.boot_patch_level, u32)?,
            device_unique_attestation: Some(data.device_unique_attestation.is_some()),
        })
    }
}

#[derive(RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq, Eq)]
pub struct BoundedRootOfTrust {
    pub verified_boot_key: VerifiedBootKey,
    pub device_locked: bool,
    pub verified_boot_state: VerifiedBootState,
    pub verified_boot_hash: Option<VerifiedBootHash>,
}

impl TryFrom<asn::RootOfTrustV1V2<'_>> for BoundedRootOfTrust {
    type Error = ();

    fn try_from(data: asn::RootOfTrustV1V2) -> Result<Self, Self::Error> {
        Ok(BoundedRootOfTrust {
            verified_boot_key: VerifiedBootKey::try_from(data.verified_boot_key.to_vec())?,
            device_locked: data.device_locked,
            verified_boot_state: data.verified_boot_state.into(),
            verified_boot_hash: None,
        })
    }
}

impl TryFrom<asn::RootOfTrust<'_>> for BoundedRootOfTrust {
    type Error = ();

    fn try_from(data: asn::RootOfTrust) -> Result<Self, Self::Error> {
        Ok(BoundedRootOfTrust {
            verified_boot_key: VerifiedBootKey::try_from(data.verified_boot_key.to_vec())?,
            device_locked: data.device_locked,
            verified_boot_state: data.verified_boot_state.into(),
            verified_boot_hash: Some(VerifiedBootHash::try_from(
                data.verified_boot_hash.to_vec(),
            )?),
        })
    }
}

#[derive(RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq, Eq)]
pub enum VerifiedBootState {
    Verified,
    SelfSigned,
    Unverified,
    Failed,
}

impl From<asn::VerifiedBootState> for VerifiedBootState {
    fn from(data: asn::VerifiedBootState) -> Self {
        match data.value() {
            0 => VerifiedBootState::Verified,
            1 => VerifiedBootState::SelfSigned,
            2 => VerifiedBootState::Unverified,
            _ => VerifiedBootState::Failed,
        }
    }
}
