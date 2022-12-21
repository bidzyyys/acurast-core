use frame_support::{
    dispatch::RawOrigin,
    sp_runtime::traits::{AccountIdConversion, Get, StaticLookup},
    traits::{fungibles, Contains},
};
use sp_std::{marker::PhantomData, result::Result};
use xcm::latest::{Error as XcmError, MultiAsset, MultiLocation, Result as XcmResult};
use xcm_builder::{FungiblesMutateAdapter, FungiblesTransferAdapter};
use xcm_executor::traits::{Convert, MatchesFungibles, TransactAsset};

use crate::Config;

pub trait MultiAssetConverter<AssetId> {
    type Error;

    fn try_convert(asset: &MultiAsset) -> Result<AssetId, Self::Error>;
}

/// wrapper around FungiblesAdapter. It proxies to it and just on deposit_asset if it failed due to
/// the asset not being created, then creates it and calls the adapter again
pub struct AssetTransactor<
    Runtime,
    Assets,
    Matcher,
    AccountIdConverter,
    AccountId,
    CheckAsset,
    CheckingAccount,
    AssetConverter,
>(
    PhantomData<(
        Runtime,
        Assets,
        Matcher,
        AccountIdConverter,
        AccountId,
        CheckAsset,
        CheckingAccount,
        AssetConverter,
    )>,
);

impl<
        Runtime: Config + pallet_assets::Config,
        Assets: fungibles::Mutate<AccountId>
            + fungibles::Transfer<AccountId>
            + fungibles::Create<AccountId>,
        Matcher: MatchesFungibles<Assets::AssetId, Assets::Balance>,
        AccountIdConverter: Convert<MultiLocation, AccountId>,
        AccountId: Clone, // can't get away without it since Currency is generic over it.
        CheckAsset: Contains<Assets::AssetId>,
        CheckingAccount: Get<AccountId>,
        AssetConverter: MultiAssetConverter<Runtime::AssetIdParameter, Error = XcmError>,
    > TransactAsset
    for AssetTransactor<
        Runtime,
        Assets,
        Matcher,
        AccountIdConverter,
        AccountId,
        CheckAsset,
        CheckingAccount,
        AssetConverter,
    >
{
    fn can_check_in(origin: &MultiLocation, what: &MultiAsset) -> XcmResult {
        FungiblesMutateAdapter::<
            Assets,
            Matcher,
            AccountIdConverter,
            AccountId,
            CheckAsset,
            CheckingAccount,
        >::can_check_in(origin, what)
    }

    fn check_in(origin: &MultiLocation, what: &MultiAsset) {
        FungiblesMutateAdapter::<
            Assets,
            Matcher,
            AccountIdConverter,
            AccountId,
            CheckAsset,
            CheckingAccount,
        >::check_in(origin, what)
    }

    fn check_out(dest: &MultiLocation, what: &MultiAsset) {
        FungiblesMutateAdapter::<
            Assets,
            Matcher,
            AccountIdConverter,
            AccountId,
            CheckAsset,
            CheckingAccount,
        >::check_out(dest, what)
    }

    fn deposit_asset(what: &MultiAsset, who: &MultiLocation) -> XcmResult {
        FungiblesMutateAdapter::<
            Assets,
            Matcher,
            AccountIdConverter,
            AccountId,
            CheckAsset,
            CheckingAccount,
        >::deposit_asset(what, who)
        .or_else(|_| {
            // asset might not have been created. Try creating it and give it again to FungiblesMutateAdapter
            let asset_id = AssetConverter::try_convert(&what)?;
            let pallet_assets_account: <Runtime as frame_system::Config>::AccountId =
                <Runtime as crate::Config>::PalletId::get().into_account_truncating();
            let raw_origin = RawOrigin::<<Runtime as frame_system::Config>::AccountId>::Signed(
                pallet_assets_account.clone(),
            );
            let pallet_origin: <Runtime as frame_system::Config>::RuntimeOrigin = raw_origin.into();

            pallet_assets::Pallet::<Runtime>::create(
                pallet_origin,
                asset_id,
                <Runtime as frame_system::Config>::Lookup::unlookup(pallet_assets_account),
                <Runtime as pallet_assets::Config>::Balance::from(1u32),
            )
            .map_err(|_| XcmError::FailedToTransactAsset("unable to create asset"))?;

            // try depositing again
            FungiblesMutateAdapter::<
                Assets,
                Matcher,
                AccountIdConverter,
                AccountId,
                CheckAsset,
                CheckingAccount,
            >::deposit_asset(what, who)
        })
    }

    fn withdraw_asset(
        what: &MultiAsset,
        who: &MultiLocation,
    ) -> Result<xcm_executor::Assets, XcmError> {
        FungiblesMutateAdapter::<
            Assets,
            Matcher,
            AccountIdConverter,
            AccountId,
            CheckAsset,
            CheckingAccount,
        >::withdraw_asset(what, who)
    }

    fn internal_transfer_asset(
        what: &MultiAsset,
        from: &MultiLocation,
        to: &MultiLocation,
    ) -> Result<xcm_executor::Assets, XcmError> {
        FungiblesTransferAdapter::<Assets, Matcher, AccountIdConverter, AccountId>::internal_transfer_asset(
            what, from, to,
        )
    }
}
