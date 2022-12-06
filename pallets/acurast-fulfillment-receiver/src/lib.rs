#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod traits;

pub use acurast_common::{is_valid_script, Fulfillment, Script};
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use acurast_common::Fulfillment;
    use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
    use frame_system::{ensure_signed, pallet_prelude::OriginFor};
    use sp_std::prelude::*;

    use crate::traits::*;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// Handler to notify the runtime when a new fulfillment is received.
        type OnFulfillment: OnFulfillment<Self>;
        /// Weight Info for extrinsics.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        FulfillReceived(T::AccountId, Fulfillment),
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        FulfillmentRejected,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a fulfillment for an acurast job.
        #[pallet::weight(T::WeightInfo::fulfill())]
        pub fn fulfill(
            origin: OriginFor<T>,
            fulfillment: Fulfillment,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            // Notify the runtime about the fulfillment.
            let info = T::OnFulfillment::on_fulfillment(who.clone(), fulfillment.clone())?;
            Self::deposit_event(Event::FulfillReceived(who, fulfillment));
            Ok(info)
        }
    }
}