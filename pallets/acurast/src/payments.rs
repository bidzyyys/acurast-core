use xcm::latest::prelude::*;
use super::Config;
use super::xcm_adapters::get_statemint_asset;
use sp_runtime::traits::{AccountIdConversion, Get, StaticLookup};
use frame_support::traits::OriginTrait;
use frame_support::dispatch::{Dispatchable, RawOrigin};

pub trait LockAndPayAsset<T: Config> {
    fn lock_asset(asset: MultiAsset, owner: <T::Lookup as StaticLookup>::Source) -> Result<(), ()>;

    fn pay_asset(asset: MultiAsset, target: <T::Lookup as StaticLookup>::Source) -> Result<(), ()>;
}

pub struct StatemintAssetTransactor;
impl<T: Config> LockAndPayAsset<T> for StatemintAssetTransactor where
    T::AssetId: TryFrom<u32>,
    T::Balance: TryFrom<u128>
{
    fn lock_asset(asset: MultiAsset, owner: <T::Lookup as StaticLookup>::Source) -> Result<(), ()> {
        let pallet_account: T::AccountId = T::PalletId::get().into_account_truncating();
        let raw_origin= RawOrigin::<T::AccountId>::Signed(pallet_account.clone());
        let pallet_origin: T::Origin = raw_origin.into();

        let (id, amount) = get_statemint_asset(&asset).map_err(|_| ())?;
        let (id, amount): (T::AssetId, T::Balance) = match (id.try_into(), amount.try_into()) {
                (Ok(id), Ok(amount)) => (id, amount),
                _ => return Err(())
        };

        // transfer funds from caller to pallet account for holding until fulfill is called
        // this is a privileged operation, hence the force_transfer call.
        // we could do an approve_transfer first, but this would require the assets pallet being
        // public which we can't do at the moment due to our statemint assets 1 to 1 integration
        let extrinsic_call = pallet_assets::Pallet::<T>::force_transfer(
            pallet_origin,
            id,
            owner,
           T::Lookup::unlookup(pallet_account),
            amount
        );

        match extrinsic_call {
            Ok(_) => Ok(()),
            Err(_) => Err(())
        }

    }

    fn pay_asset(asset: MultiAsset, target: <T::Lookup as StaticLookup>::Source) -> Result<(), ()> {
        let pallet_account: T::AccountId = T::PalletId::get().into_account_truncating();
        let raw_origin= RawOrigin::<T::AccountId>::Signed(pallet_account);
        let pallet_origin: T::Origin = raw_origin.into();

        let (id, amount) = get_statemint_asset(&asset).map_err(|_| ())?;;
        let (id, amount): (T::AssetId, T::Balance) = match (id.try_into(), amount.try_into()) {
            (Ok(id), Ok(amount)) => (id, amount),
            _ => return Err(())
        };

        let extrinsic_call = pallet_assets::Pallet::<T>::transfer(
            pallet_origin,
            id,
            target,
            amount
        );

        match extrinsic_call {
            Ok(_) => Ok(()),
            Err(_) => Err(())
        }
    }
}