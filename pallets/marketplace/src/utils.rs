use crate::Config;
use pallet_acurast::JobRegistrationFor;
use sp_std::prelude::*;

/// Checks if a consumer is whitelisted/
pub(crate) fn is_consumer_whitelisted<T: Config>(
    consumer: &T::AccountId,
    allowed_consumers: &Option<Vec<T::AccountId>>,
) -> bool {
    allowed_consumers
        .as_ref()
        .map(|allowed_consumers| {
            allowed_consumers
                .iter()
                .any(|allowed_consumer| allowed_consumer == consumer)
        })
        .unwrap_or(true)
}

/// Checks if a source/processor is whitelisted
pub fn is_source_whitelisted<T: Config>(
    source: &T::AccountId,
    registration: &JobRegistrationFor<T>,
) -> bool {
    registration
        .allowed_sources
        .as_ref()
        .map(|allowed_sources| {
            allowed_sources
                .iter()
                .any(|allowed_source| allowed_source == source)
        })
        .unwrap_or(true)
}
