use std::marker::PhantomData;

use frame_support::traits::OriginTrait;
use pallet_acurast_marketplace::Reward;
use scale_info::TypeInfo;
use sp_core::*;
use sp_std::prelude::*;
use xcm::latest::{Junction, MultiLocation, OriginKind};
use xcm::prelude::*;
use xcm_executor::traits::ConvertOrigin;

pub type AcurastAssetId = AssetId;
pub type InternalAssetId = u32;
pub type AcurastAssetAmount = u128;

#[derive(Clone, Eq, PartialEq, Debug, Encode, Decode, TypeInfo)]
pub struct AcurastAsset(pub MultiAsset);

impl Reward for AcurastAsset {
    type AssetId = AcurastAssetId;
    type AssetAmount = AcurastAssetAmount;
    type Error = ();

    fn with_amount(&mut self, amount: Self::AssetAmount) -> Result<&Self, Self::Error> {
        self.0 = MultiAsset {
            id: self.0.id.clone(),
            fun: Fungible(amount),
        };
        Ok(self)
    }

    fn try_get_asset_id(&self) -> Result<Self::AssetId, Self::Error> {
        Ok(self.0.id.clone())
    }

    fn try_get_amount(&self) -> Result<Self::AssetAmount, Self::Error> {
        match self.0.fun {
            Fungible(amount) => Ok(amount),
            _ => Err(()),
        }
    }
}

pub mod acurast_runtime {
    use frame_support::{
        construct_runtime, parameter_types,
        sp_runtime::{testing::Header, traits::AccountIdLookup, AccountId32},
        traits::{AsEnsureOriginWithArg, Everything, Nothing},
        PalletId,
    };
    use pallet_xcm::XcmPassthrough;
    use polkadot_parachain::primitives::Sibling;
    use sp_core::*;
    use sp_runtime::DispatchError;
    use sp_std::prelude::*;
    use xcm::latest::prelude::*;
    use xcm_builder::{
        AccountId32Aliases, AllowUnpaidExecutionFrom, CurrencyAdapter as XcmCurrencyAdapter,
        EnsureXcmOrigin, FixedRateOfFungible, FixedWeightBounds, IsConcrete, LocationInverter,
        NativeAsset, ParentIsPreset, SiblingParachainConvertsVia, SignedAccountId32AsNative,
        SignedToAccountId32, SovereignSignedViaLocation,
    };
    use xcm_executor::XcmExecutor;

    pub use pallet_acurast;
    use pallet_acurast_assets::traits::AssetValidator;
    pub use pallet_acurast_marketplace;
    use pallet_acurast_marketplace::{AssetBarrier, AssetRewardManager, JobRequirements};

    use super::{AcurastAsset, AcurastAssetAmount, AcurastAssetId, InternalAssetId};

    pub type AccountId = AccountId32;
    pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;
    pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
    pub type Block = frame_system::mocking::MockBlock<Runtime>;
    pub type LocationToAccountId = (
        ParentIsPreset<AccountId>,
        SiblingParachainConvertsVia<Sibling, AccountId>,
        AccountId32Aliases<RelayNetwork, AccountId>,
    );
    pub type LocalAssetTransactor =
        XcmCurrencyAdapter<Balances, IsConcrete<KsmLocation>, LocationToAccountId, AccountId, ()>;
    pub type XcmRouter = crate::tests::ParachainXcmRouter<MsgQueue>;
    pub type Barrier = AllowUnpaidExecutionFrom<Everything>;
    pub type XcmOriginToCallOrigin = (
        SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
        SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
        // TODO: safety check of signature
        super::SignedAccountId32FromXcm<RuntimeOrigin>,
        XcmPassthrough<RuntimeOrigin>,
    );

    pub struct AcurastBarrier;

    impl AssetBarrier<AcurastAsset> for AcurastBarrier {
        fn can_use_asset(_asset: &AcurastAsset) -> bool {
            true
        }
    }

    pub struct PassAllAssets {}
    impl<AssetId> AssetValidator<AssetId> for PassAllAssets {
        type Error = DispatchError;

        fn validate(_: &AssetId) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    pub const MILLISECS_PER_BLOCK: u64 = 12000;
    pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;
    pub const UNIT: AcurastAssetAmount = 1_000_000;
    pub const MICROUNIT: AcurastAssetAmount = 1;

    construct_runtime!(
        pub enum Runtime where
            Block = Block,
            NodeBlock = Block,
            UncheckedExtrinsic = UncheckedExtrinsic,
        {
            System: frame_system::{Pallet, Call, Storage, Config, Event<T>} = 0,
            Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
            Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
            Assets: pallet_assets::{Pallet, Storage, Event<T>, Config<T>}, // hide calls since they get proxied by `pallet_acurast_assets`
            AcurastAssets: pallet_acurast_assets::{Pallet, Storage, Event<T>, Config<T>, Call},
            ParachainInfo: parachain_info::{Pallet, Storage, Config},
            MsgQueue: super::mock_msg_queue::{Pallet, Storage, Event<T>},
            PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin},
            Acurast: pallet_acurast::{Pallet, Call, Storage, Event<T>} = 40,
            AcurastMarketplace: pallet_acurast_marketplace::{Pallet, Call, Storage, Event<T>} = 41,
        }
    );

    parameter_types! {
        pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
        pub const IsRelay: bool = false;
        pub const AcurastPalletId: PalletId = PalletId(*b"acrstpid");
        pub const ReportTolerance: u64 = 12000;
    }
    parameter_types! {
        pub const BlockHashCount: u64 = 250;
    }
    parameter_types! {
        pub ExistentialDeposit: AcurastAssetAmount = 1;
        pub const MaxLocks: u32 = 50;
        pub const MaxReserves: u32 = 50;
    }
    parameter_types! {
        pub const KsmLocation: MultiLocation = MultiLocation::parent();
        pub const RelayNetwork: NetworkId = NetworkId::Kusama;
        pub Ancestry: MultiLocation = Parachain(MsgQueue::parachain_id().into()).into();
    }
    parameter_types! {
        pub const UnitWeightCost: u64 = 1;
        pub KsmPerSecond: (AssetId, u128) = (Concrete(Parent.into()), 1);
        pub const MaxInstructions: u32 = 100;
    }

    pub struct XcmConfig;

    impl xcm_executor::Config for XcmConfig {
        type RuntimeCall = RuntimeCall;
        type XcmSender = XcmRouter;
        type AssetTransactor = LocalAssetTransactor;
        type OriginConverter = XcmOriginToCallOrigin;
        type IsReserve = NativeAsset;
        type IsTeleporter = ();
        type LocationInverter = LocationInverter<Ancestry>;
        type Barrier = Barrier;
        type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
        type Trader = FixedRateOfFungible<KsmPerSecond, ()>;
        type ResponseHandler = ();
        type AssetTrap = ();
        type AssetClaims = ();
        type SubscriptionService = ();
    }

    impl pallet_balances::Config for Runtime {
        type Balance = AcurastAssetAmount;
        type DustRemoval = ();
        type RuntimeEvent = RuntimeEvent;
        type ExistentialDeposit = ExistentialDeposit;
        type AccountStore = System;
        type WeightInfo = ();
        type MaxLocks = MaxLocks;
        type MaxReserves = MaxReserves;
        type ReserveIdentifier = [u8; 8];
    }

    impl frame_system::Config for Runtime {
        type BaseCallFilter = Everything;
        type BlockWeights = ();
        type BlockLength = ();
        type RuntimeOrigin = RuntimeOrigin;
        type RuntimeCall = RuntimeCall;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = frame_support::sp_runtime::traits::BlakeTwo256;
        type AccountId = AccountId;
        type Lookup = AccountIdLookup<AccountId, ()>;
        type Header = Header;
        type RuntimeEvent = RuntimeEvent;
        type BlockHashCount = BlockHashCount;
        type DbWeight = ();
        type Version = ();
        type PalletInfo = PalletInfo;
        type AccountData = pallet_balances::AccountData<AcurastAssetAmount>;
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type SystemWeightInfo = ();
        type SS58Prefix = ();
        type OnSetCode = ();
        type MaxConsumers = frame_support::traits::ConstU32<16>;
    }

    impl parachain_info::Config for Runtime {}

    impl pallet_timestamp::Config for Runtime {
        type Moment = u64;
        type OnTimestampSet = ();
        type MinimumPeriod = MinimumPeriod;
        type WeightInfo = ();
    }

    impl pallet_assets::Config for Runtime {
        type RuntimeEvent = RuntimeEvent;
        type Balance = AcurastAssetAmount;
        type AssetId = InternalAssetId;
        type AssetIdParameter = codec::Compact<InternalAssetId>;
        type Currency = Balances;
        type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId>>;
        type ForceOrigin = frame_system::EnsureRoot<Self::AccountId>;
        type AssetDeposit = frame_support::traits::ConstU128<0>;
        type AssetAccountDeposit = frame_support::traits::ConstU128<0>;
        type MetadataDepositBase = frame_support::traits::ConstU128<{ UNIT }>;
        type MetadataDepositPerByte = frame_support::traits::ConstU128<{ 10 * MICROUNIT }>;
        type ApprovalDeposit = frame_support::traits::ConstU128<{ 10 * MICROUNIT }>;
        type StringLimit = frame_support::traits::ConstU32<50>;
        type Freezer = ();
        type Extra = ();
        type WeightInfo = ();
        type RemoveItemsLimit = ();
    }

    impl pallet_acurast_assets::Config for Runtime {
        type RuntimeEvent = RuntimeEvent;
        type WeightInfo = ();
    }

    pub struct FeeManagerImpl;

    impl pallet_acurast_marketplace::FeeManager for FeeManagerImpl {
        fn get_fee_percentage() -> sp_runtime::Percent {
            sp_runtime::Percent::from_percent(30)
        }

        fn get_matcher_percentage() -> sp_runtime::Percent {
            sp_runtime::Percent::from_percent(10)
        }

        fn pallet_id() -> PalletId {
            PalletId(*b"acurfees")
        }
    }

    impl pallet_acurast::Config for Runtime {
        type RuntimeEvent = RuntimeEvent;
        type RegistrationExtra = JobRequirements<AcurastAsset, AccountId>;
        type MaxAllowedSources = frame_support::traits::ConstU16<1000>;
        type PalletId = AcurastPalletId;
        type RevocationListUpdateBarrier = ();
        type KeyAttestationBarrier = ();
        type UnixTime = pallet_timestamp::Pallet<Runtime>;
        type JobHooks = pallet_acurast_marketplace::Pallet<Runtime>;
        type WeightInfo = pallet_acurast::weights::WeightInfo<Runtime>;
    }

    impl pallet_acurast_marketplace::Config for Runtime {
        type RuntimeEvent = RuntimeEvent;
        type RegistrationExtra = JobRequirements<AcurastAsset, AccountId>;
        type PalletId = AcurastPalletId;
        type ReportTolerance = ReportTolerance;
        type AssetId = AcurastAssetId;
        type AssetAmount = AcurastAssetAmount;
        type RewardManager = AssetRewardManager<AcurastAsset, AcurastBarrier, FeeManagerImpl>;
        type AssetValidator = PassAllAssets;
        type WeightInfo = pallet_acurast_marketplace::weights::Weights<Runtime>;
    }

    impl pallet_xcm::Config for Runtime {
        type RuntimeEvent = RuntimeEvent;
        type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
        type XcmRouter = XcmRouter;
        type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
        type XcmExecuteFilter = Everything;
        type XcmExecutor = XcmExecutor<XcmConfig>;
        type XcmTeleportFilter = Nothing;
        type XcmReserveTransferFilter = Everything;
        type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
        type LocationInverter = LocationInverter<Ancestry>;
        type RuntimeOrigin = RuntimeOrigin;
        type RuntimeCall = RuntimeCall;
        const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
        type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
    }

    impl super::mock_msg_queue::Config for Runtime {
        type RuntimeEvent = RuntimeEvent;
        type XcmExecutor = XcmExecutor<XcmConfig>;
    }
}

pub mod proxy_runtime {
    use frame_support::{
        construct_runtime, parameter_types,
        traits::{Everything, Nothing},
    };
    use pallet_xcm::XcmPassthrough;
    use polkadot_parachain::primitives::Sibling;
    use sp_core::H256;
    use sp_runtime::{testing::Header, traits::AccountIdLookup, AccountId32};
    use sp_std::prelude::*;
    use xcm::latest::prelude::*;
    use xcm_builder::{
        AccountId32Aliases, AllowUnpaidExecutionFrom, CurrencyAdapter as XcmCurrencyAdapter,
        EnsureXcmOrigin, FixedRateOfFungible, FixedWeightBounds, IsConcrete, LocationInverter,
        NativeAsset, ParentIsPreset, SiblingParachainConvertsVia, SignedAccountId32AsNative,
        SignedToAccountId32, SovereignSignedViaLocation,
    };
    use xcm_executor::{Config, XcmExecutor};

    use pallet_acurast_marketplace::JobRequirements;

    use crate::mock::{AcurastAsset, AcurastAssetAmount, AcurastAssetId};

    pub type AccountId = AccountId32;
    pub type LocationToAccountId = (
        ParentIsPreset<AccountId>,
        SiblingParachainConvertsVia<Sibling, AccountId>,
        AccountId32Aliases<RelayNetwork, AccountId>,
    );
    pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;
    pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
    pub type Block = frame_system::mocking::MockBlock<Runtime>;
    pub type XcmOriginToCallOrigin = (
        SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
        SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
        // TODO: safety check of signature
        super::SignedAccountId32FromXcm<RuntimeOrigin>,
        XcmPassthrough<RuntimeOrigin>,
    );
    pub type LocalAssetTransactor =
        XcmCurrencyAdapter<Balances, IsConcrete<KsmLocation>, LocationToAccountId, AccountId, ()>;
    pub type XcmRouter = crate::tests::ParachainXcmRouter<MsgQueue>;
    pub type Barrier = AllowUnpaidExecutionFrom<Everything>;

    pub struct XcmConfig;

    impl Config for XcmConfig {
        type RuntimeCall = RuntimeCall;
        type XcmSender = XcmRouter;
        type AssetTransactor = LocalAssetTransactor;
        type OriginConverter = XcmOriginToCallOrigin;
        type IsReserve = NativeAsset;
        type IsTeleporter = ();
        type LocationInverter = LocationInverter<Ancestry>;
        type Barrier = Barrier;
        type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
        type Trader = FixedRateOfFungible<KsmPerSecond, ()>;
        type ResponseHandler = ();
        type AssetTrap = ();
        type AssetClaims = ();
        type SubscriptionService = ();
    }

    pub const MILLISECS_PER_BLOCK: u64 = 12000;
    pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

    construct_runtime!(
        pub enum Runtime where
            Block = Block,
            NodeBlock = Block,
            UncheckedExtrinsic = UncheckedExtrinsic,
        {
            System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
            Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
            MsgQueue: super::mock_msg_queue::{Pallet, Storage, Event<T>},
            PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin},
            AcurastProxy: crate::{Pallet, Call, Event<T>} = 34,
        }
    );

    parameter_types! {
    pub const BlockHashCount: u64 = 250;
    }
    parameter_types! {
        pub ExistentialDeposit: AcurastAssetAmount = 1;
        pub const MaxLocks: u32 = 50;
        pub const MaxReserves: u32 = 50;
    }
    parameter_types! {
        pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
        pub const IsRelay: bool = false;
        pub Admins: Vec<AccountId> = vec![];
    }
    parameter_types! {
        pub const UnitWeightCost: u64 = 1;
        pub KsmPerSecond: (AssetId, u128) = (Concrete(Parent.into()), 1);
        pub const MaxInstructions: u32 = 100;
    }
    parameter_types! {
        pub const AcurastParachainId: u32 = 2000;
        pub const AcurastPalletId: u8 = 40;
        pub const AcurastMarketplacePalletId: u8 = 41;
    }
    parameter_types! {
        pub const KsmLocation: MultiLocation = MultiLocation::parent();
        pub const RelayNetwork: NetworkId = NetworkId::Kusama;
        pub Ancestry: MultiLocation = Parachain(MsgQueue::parachain_id().into()).into();
    }

    impl frame_system::Config for Runtime {
        type BaseCallFilter = Everything;
        type BlockWeights = ();
        type BlockLength = ();
        type RuntimeOrigin = RuntimeOrigin;
        type RuntimeCall = RuntimeCall;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = frame_support::sp_runtime::traits::BlakeTwo256;
        type AccountId = AccountId;
        type Lookup = AccountIdLookup<AccountId, ()>;
        type Header = Header;
        type RuntimeEvent = RuntimeEvent;
        type BlockHashCount = BlockHashCount;
        type DbWeight = ();
        type Version = ();
        type PalletInfo = PalletInfo;
        type AccountData = pallet_balances::AccountData<AcurastAssetAmount>;
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type SystemWeightInfo = ();
        type SS58Prefix = ();
        type OnSetCode = ();
        type MaxConsumers = frame_support::traits::ConstU32<16>;
    }

    impl pallet_balances::Config for Runtime {
        type Balance = AcurastAssetAmount;
        type DustRemoval = ();
        type RuntimeEvent = RuntimeEvent;
        type ExistentialDeposit = ExistentialDeposit;
        type AccountStore = System;
        type WeightInfo = ();
        type MaxLocks = MaxLocks;
        type MaxReserves = MaxReserves;
        type ReserveIdentifier = [u8; 8];
    }

    impl super::mock_msg_queue::Config for Runtime {
        type RuntimeEvent = RuntimeEvent;
        type XcmExecutor = XcmExecutor<XcmConfig>;
    }

    impl pallet_xcm::Config for Runtime {
        type RuntimeEvent = RuntimeEvent;
        type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
        type XcmRouter = XcmRouter;
        type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
        type XcmExecuteFilter = Everything;
        type XcmExecutor = XcmExecutor<XcmConfig>;
        type XcmTeleportFilter = Nothing;
        type XcmReserveTransferFilter = Everything;
        type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
        type LocationInverter = LocationInverter<Ancestry>;
        type RuntimeOrigin = RuntimeOrigin;
        type RuntimeCall = RuntimeCall;
        const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
        type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
    }

    impl crate::Config for Runtime {
        type RuntimeEvent = RuntimeEvent;
        type RegistrationExtra = JobRequirements<AcurastAsset, AccountId>;
        type AssetId = AcurastAssetId;
        type AssetAmount = AcurastAssetAmount;
        type XcmSender = XcmRouter;
        type AcurastPalletId = AcurastPalletId;
        type AcurastMarketplacePalletId = AcurastMarketplacePalletId;
        type AcurastParachainId = AcurastParachainId;
    }

    impl pallet_timestamp::Config for Runtime {
        type Moment = u64;
        type OnTimestampSet = ();
        type MinimumPeriod = MinimumPeriod;
        type WeightInfo = ();
    }
}

pub mod relay_chain {
    use frame_support::{
        construct_runtime, parameter_types,
        sp_runtime::{testing::Header, traits::IdentityLookup, AccountId32},
        traits::{Everything, Nothing},
    };
    use polkadot_parachain::primitives::Id as ParaId;
    use polkadot_runtime_parachains::{configuration, origin, shared, ump};
    use sp_core::H256;
    use xcm::latest::prelude::*;
    use xcm_builder::{
        AccountId32Aliases, AllowUnpaidExecutionFrom, ChildParachainAsNative,
        ChildParachainConvertsVia, ChildSystemParachainAsSuperuser,
        CurrencyAdapter as XcmCurrencyAdapter, FixedRateOfFungible, FixedWeightBounds, IsConcrete,
        LocationInverter, SignedAccountId32AsNative, SignedToAccountId32,
        SovereignSignedViaLocation,
    };
    use xcm_executor::{Config, XcmExecutor};

    use crate::mock::AcurastAssetAmount;

    pub type AccountId = AccountId32;
    pub type SovereignAccountOf = (
        ChildParachainConvertsVia<ParaId, AccountId>,
        AccountId32Aliases<KusamaNetwork, AccountId>,
    );
    pub type LocalAssetTransactor =
        XcmCurrencyAdapter<Balances, IsConcrete<KsmLocation>, SovereignAccountOf, AccountId, ()>;
    pub type LocalOriginConverter = (
        SovereignSignedViaLocation<SovereignAccountOf, RuntimeOrigin>,
        ChildParachainAsNative<origin::Origin, RuntimeOrigin>,
        SignedAccountId32AsNative<KusamaNetwork, RuntimeOrigin>,
        ChildSystemParachainAsSuperuser<ParaId, RuntimeOrigin>,
    );
    pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, KusamaNetwork>;
    pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
    pub type Block = frame_system::mocking::MockBlock<Runtime>;
    pub type XcmRouter = crate::tests::RelayChainXcmRouter;
    pub type Barrier = AllowUnpaidExecutionFrom<Everything>;

    pub struct XcmConfig;

    impl Config for XcmConfig {
        type RuntimeCall = RuntimeCall;
        type XcmSender = XcmRouter;
        type AssetTransactor = LocalAssetTransactor;
        type OriginConverter = LocalOriginConverter;
        type IsReserve = ();
        type IsTeleporter = ();
        type LocationInverter = LocationInverter<Ancestry>;
        type Barrier = Barrier;
        type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
        type Trader = FixedRateOfFungible<KsmPerSecond, ()>;
        type ResponseHandler = ();
        type AssetTrap = ();
        type AssetClaims = ();
        type SubscriptionService = ();
    }

    construct_runtime!(
        pub enum Runtime where
            Block = Block,
            NodeBlock = Block,
            UncheckedExtrinsic = UncheckedExtrinsic,
        {
            System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
            Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
            ParasOrigin: origin::{Pallet, Origin},
            ParasUmp: ump::{Pallet, Call, Storage, Event},
            XcmPallet: pallet_xcm::{Pallet, Call, Storage, Event<T>, Origin},
        }
    );

    parameter_types! {
        pub const KsmLocation: MultiLocation = Here.into();
        pub const KusamaNetwork: NetworkId = NetworkId::Kusama;
        pub const AnyNetwork: NetworkId = NetworkId::Any;
        pub Ancestry: MultiLocation = Here.into();
        pub UnitWeightCost: u64 = 1_000;
    }
    parameter_types! {
        pub const BaseXcmWeight: u64 = 1_000;
        pub KsmPerSecond: (AssetId, u128) = (Concrete(KsmLocation::get()), 1);
        pub const MaxInstructions: u32 = 100;
    }
    parameter_types! {
        pub const FirstMessageFactorPercent: u64 = 100;
    }
    parameter_types! {
        pub ExistentialDeposit: AcurastAssetAmount = 1;
        pub const MaxLocks: u32 = 50;
        pub const MaxReserves: u32 = 50;
    }
    parameter_types! {
        pub const BlockHashCount: u64 = 250;
    }

    impl frame_system::Config for Runtime {
        type BaseCallFilter = Everything;
        type BlockWeights = ();
        type BlockLength = ();
        type RuntimeOrigin = RuntimeOrigin;
        type RuntimeCall = RuntimeCall;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = frame_support::sp_runtime::traits::BlakeTwo256;
        type AccountId = AccountId;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type RuntimeEvent = RuntimeEvent;
        type BlockHashCount = BlockHashCount;
        type DbWeight = ();
        type Version = ();
        type PalletInfo = PalletInfo;
        type AccountData = pallet_balances::AccountData<AcurastAssetAmount>;
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type SystemWeightInfo = ();
        type SS58Prefix = ();
        type OnSetCode = ();
        type MaxConsumers = frame_support::traits::ConstU32<16>;
    }

    impl pallet_balances::Config for Runtime {
        type Balance = AcurastAssetAmount;
        type DustRemoval = ();
        type RuntimeEvent = RuntimeEvent;
        type ExistentialDeposit = ExistentialDeposit;
        type AccountStore = System;
        type WeightInfo = ();
        type MaxLocks = MaxLocks;
        type MaxReserves = MaxReserves;
        type ReserveIdentifier = [u8; 8];
    }

    impl shared::Config for Runtime {}

    impl configuration::Config for Runtime {
        type WeightInfo = configuration::TestWeightInfo;
    }

    impl pallet_xcm::Config for Runtime {
        type RuntimeEvent = RuntimeEvent;
        type SendXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
        type XcmRouter = XcmRouter;
        // Anyone can execute XCM messages locally...
        type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
        type XcmExecuteFilter = Nothing;
        type XcmExecutor = XcmExecutor<XcmConfig>;
        type XcmTeleportFilter = Everything;
        type XcmReserveTransferFilter = Everything;
        type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
        type LocationInverter = LocationInverter<Ancestry>;
        type RuntimeOrigin = RuntimeOrigin;
        type RuntimeCall = RuntimeCall;
        const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
        type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
    }

    impl ump::Config for Runtime {
        type RuntimeEvent = RuntimeEvent;
        type UmpSink = ump::XcmSink<XcmExecutor<XcmConfig>, Runtime>;
        type FirstMessageFactorPercent = FirstMessageFactorPercent;
        type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
        type WeightInfo = ump::TestWeightInfo;
    }

    impl origin::Config for Runtime {}
}

#[frame_support::pallet]
pub mod mock_msg_queue {
    use frame_support::pallet_prelude::*;
    use polkadot_parachain::primitives::{
        DmpMessageHandler, XcmpMessageFormat, XcmpMessageHandler,
    };
    use sp_runtime::traits::Hash;
    use xcm::latest::{ExecuteXcm, Outcome, Parent, Xcm};
    use xcm::prelude::{Parachain, XcmError};
    use xcm::VersionedXcm;
    use xcm_simulator::{ParaId, RelayBlockNumber};

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type XcmExecutor: ExecuteXcm<Self::RuntimeCall>;
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {}

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn parachain_id)]
    pub(super) type ParachainId<T: Config> = StorageValue<_, ParaId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn received_dmp)]
    /// A queue of received DMP messages
    pub(super) type ReceivedDmp<T: Config> = StorageValue<_, Vec<Xcm<T::RuntimeCall>>, ValueQuery>;

    impl<T: Config> Get<ParaId> for Pallet<T> {
        fn get() -> ParaId {
            Self::parachain_id()
        }
    }

    pub type MessageId = [u8; 32];

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        // XCMP
        /// Some XCM was executed OK.
        Success(Option<T::Hash>),
        /// Some XCM failed.
        Fail(Option<T::Hash>, XcmError),
        /// Bad XCM version used.
        BadVersion(Option<T::Hash>),
        /// Bad XCM format used.
        BadFormat(Option<T::Hash>),

        // DMP
        /// Downward message is invalid XCM.
        InvalidFormat(MessageId),
        /// Downward message is unsupported version of XCM.
        UnsupportedVersion(MessageId),
        /// Downward message executed with the given outcome.
        ExecutedDownward(MessageId, Outcome),
    }

    impl<T: Config> Pallet<T> {
        pub fn set_para_id(para_id: ParaId) {
            ParachainId::<T>::put(para_id);
        }

        fn handle_xcmp_message(
            sender: ParaId,
            _sent_at: RelayBlockNumber,
            xcm: VersionedXcm<T::RuntimeCall>,
            max_weight: Weight,
        ) -> Result<Weight, XcmError> {
            let hash = Encode::using_encoded(&xcm, T::Hashing::hash);
            let (result, event) = match Xcm::<T::RuntimeCall>::try_from(xcm) {
                Ok(xcm) => {
                    let location = (1, Parachain(sender.into()));
                    match T::XcmExecutor::execute_xcm(location, xcm, max_weight.ref_time()) {
                        Outcome::Error(e) => (Err(e.clone()), Event::Fail(Some(hash), e)),
                        Outcome::Complete(w) => {
                            (Ok(Weight::from_ref_time(w)), Event::Success(Some(hash)))
                        }
                        // As far as the caller is concerned, this was dispatched without error, so
                        // we just report the weight used.
                        Outcome::Incomplete(w, e) => {
                            (Ok(Weight::from_ref_time(w)), Event::Fail(Some(hash), e))
                        }
                    }
                }
                Err(()) => (
                    Err(XcmError::UnhandledXcmVersion),
                    Event::BadVersion(Some(hash)),
                ),
            };
            Self::deposit_event(event);
            result
        }
    }

    impl<T: Config> XcmpMessageHandler for Pallet<T> {
        fn handle_xcmp_messages<'a, I: Iterator<Item = (ParaId, RelayBlockNumber, &'a [u8])>>(
            iter: I,
            max_weight: Weight,
        ) -> Weight {
            for (sender, sent_at, data) in iter {
                let mut data_ref = data;
                let _ = XcmpMessageFormat::decode(&mut data_ref)
                    .expect("Simulator encodes with versioned xcm format; qed");

                let mut remaining_fragments = &data_ref[..];
                while !remaining_fragments.is_empty() {
                    if let Ok(xcm) =
                        VersionedXcm::<T::RuntimeCall>::decode(&mut remaining_fragments)
                    {
                        let _ = Self::handle_xcmp_message(sender, sent_at, xcm, max_weight)
                            .map_err(|e| {
                                debug_assert!(
                                    false,
                                    "Handling XCMP message returned error {:?}",
                                    e
                                );
                            });
                    } else {
                        debug_assert!(false, "Invalid incoming XCMP message data");
                    }
                }
            }
            max_weight
        }
    }

    impl<T: Config> DmpMessageHandler for Pallet<T> {
        fn handle_dmp_messages(
            iter: impl Iterator<Item = (RelayBlockNumber, Vec<u8>)>,
            limit: Weight,
        ) -> Weight {
            for (_i, (_sent_at, data)) in iter.enumerate() {
                let id = sp_io::hashing::blake2_256(&data[..]);
                let maybe_msg = VersionedXcm::<T::RuntimeCall>::decode(&mut &data[..])
                    .map(Xcm::<T::RuntimeCall>::try_from);
                match maybe_msg {
                    Err(_) => {
                        Self::deposit_event(Event::InvalidFormat(id));
                    }
                    Ok(Err(())) => {
                        Self::deposit_event(Event::UnsupportedVersion(id));
                    }
                    Ok(Ok(x)) => {
                        let outcome =
                            T::XcmExecutor::execute_xcm(Parent, x.clone(), limit.ref_time());
                        <ReceivedDmp<T>>::append(x);
                        Self::deposit_event(Event::ExecutedDownward(id, outcome));
                    }
                }
            }
            limit
        }
    }
}

pub struct SignedAccountId32FromXcm<Origin>(PhantomData<Origin>);

impl<Origin: OriginTrait> ConvertOrigin<Origin> for SignedAccountId32FromXcm<Origin>
where
    Origin::AccountId: From<[u8; 32]>,
{
    fn convert_origin(
        origin: impl Into<MultiLocation>,
        kind: OriginKind,
    ) -> Result<Origin, MultiLocation> {
        let origin = origin.into();
        log::trace!(
            target: "xcm::origin_conversion",
            "SignedAccountId32AsNative origin: {:?}, kind: {:?}",
            origin, kind,
        );
        match (kind, origin) {
            (
                OriginKind::Xcm,
                MultiLocation {
                    parents: 1,
                    interior:
                        X2(Junction::Parachain(_para_id), Junction::AccountId32 { id, network: _ }),
                },
            ) => Ok(Origin::signed(id.into())),
            (_, origin) => Err(origin),
        }
    }
}
