#![cfg(test)]

use crate::{mock::*, stub::*, Error, Event, ProcessorPairingFor, ProcessorPairingUpdateFor};
use acurast_common::ListUpdateOperation;
use frame_support::{assert_err, assert_ok, traits::fungible::Inspect};

#[test]
fn test_update_processor_pairings_succeed_1() {
    ExtBuilder::default().build().execute_with(|| {
        let (signer, processor_account) = generate_account();
        let _ = Timestamp::set(RuntimeOrigin::none(), 1657363915010);
        let timestamp = 1657363915002u128;
        let signature = generate_signature(&signer, &alice_account_id(), timestamp, 1);
        let updates = vec![ProcessorPairingUpdateFor::<Test> {
            operation: ListUpdateOperation::Add,
            item: ProcessorPairingFor::<Test>::new_with_proof(
                processor_account.clone(),
                timestamp,
                signature,
            ),
        }];
        let call = AcurastProcessorManager::update_processor_pairings(
            RuntimeOrigin::signed(alice_account_id()),
            updates.clone(),
        );
        assert_ok!(call);
        assert_eq!(Some(1), AcurastProcessorManager::last_manager_id());
        assert_eq!(
            Some(1),
            AcurastProcessorManager::manager_id_for_processor(&processor_account)
        );
        assert_eq!(
            Some(alice_account_id()),
            AcurastProcessorManager::manager_for_processor(&processor_account)
        );
        assert!(AcurastProcessorManager::managed_processors(1, &processor_account).is_some());
        let last_events = events();
        assert_eq!(
            last_events[(last_events.len() - 2)..],
            vec![
                RuntimeEvent::AcurastProcessorManager(Event::ManagerCreated(alice_account_id(), 1)),
                RuntimeEvent::AcurastProcessorManager(Event::ProcessorPairingsUpdated(
                    alice_account_id(),
                    updates
                )),
            ]
        );

        let updates = vec![ProcessorPairingUpdateFor::<Test> {
            operation: ListUpdateOperation::Remove,
            item: ProcessorPairingFor::<Test>::new(processor_account.clone()),
        }];
        let call = AcurastProcessorManager::update_processor_pairings(
            RuntimeOrigin::signed(alice_account_id()),
            updates.clone(),
        );
        assert_ok!(call);
        assert_eq!(
            None,
            AcurastProcessorManager::manager_id_for_processor(&processor_account)
        );
        assert_eq!(
            None,
            AcurastProcessorManager::manager_for_processor(&processor_account)
        );
        assert_eq!(
            events(),
            vec![RuntimeEvent::AcurastProcessorManager(
                Event::ProcessorPairingsUpdated(alice_account_id(), updates)
            ),]
        );
    });
}

#[test]
fn test_update_processor_pairings_succeed_2() {
    ExtBuilder::default().build().execute_with(|| {
        let (signer, processor_account) = generate_account();
        let _ = Timestamp::set(RuntimeOrigin::none(), 1657363915010);
        let timestamp = 1657363915002u128;
        let signature = generate_signature(&signer, &alice_account_id(), timestamp, 1);
        let updates = vec![ProcessorPairingUpdateFor::<Test> {
            operation: ListUpdateOperation::Add,
            item: ProcessorPairingFor::<Test>::new_with_proof(
                processor_account.clone(),
                timestamp,
                signature,
            ),
        }];
        let call = AcurastProcessorManager::update_processor_pairings(
            RuntimeOrigin::signed(alice_account_id()),
            updates.clone(),
        );
        assert_ok!(call);
        _ = events();

        let (signer, processor_account) = generate_account();
        let signature = generate_signature(&signer, &bob_account_id(), timestamp, 1);
        let updates = vec![ProcessorPairingUpdateFor::<Test> {
            operation: ListUpdateOperation::Add,
            item: ProcessorPairingFor::<Test>::new_with_proof(
                processor_account.clone(),
                timestamp,
                signature,
            ),
        }];
        let call = AcurastProcessorManager::update_processor_pairings(
            RuntimeOrigin::signed(bob_account_id()),
            updates.clone(),
        );
        assert_ok!(call);

        assert_eq!(Some(2), AcurastProcessorManager::last_manager_id());
        assert_eq!(
            Some(2),
            AcurastProcessorManager::manager_id_for_processor(&processor_account)
        );
        assert_eq!(
            Some(bob_account_id()),
            AcurastProcessorManager::manager_for_processor(&processor_account)
        );
        assert!(AcurastProcessorManager::managed_processors(2, &processor_account).is_some());
        let last_events = events();
        assert_eq!(
            last_events[(last_events.len() - 2)..],
            vec![
                RuntimeEvent::AcurastProcessorManager(Event::ManagerCreated(bob_account_id(), 2)),
                RuntimeEvent::AcurastProcessorManager(Event::ProcessorPairingsUpdated(
                    bob_account_id(),
                    updates
                )),
            ]
        );
    });
}

#[test]
fn test_update_processor_pairings_failure_1() {
    ExtBuilder::default().build().execute_with(|| {
        let (signer, processor_account) = generate_account();
        let _ = Timestamp::set(RuntimeOrigin::none(), 1657363915010);
        let timestamp = 1657363915002u128;
        let signature = generate_signature(&signer, &alice_account_id(), timestamp, 1);
        let updates = vec![ProcessorPairingUpdateFor::<Test> {
            operation: ListUpdateOperation::Add,
            item: ProcessorPairingFor::<Test>::new_with_proof(
                processor_account.clone(),
                1657363915003u128,
                signature,
            ),
        }];
        let call = AcurastProcessorManager::update_processor_pairings(
            RuntimeOrigin::signed(alice_account_id()),
            updates.clone(),
        );
        assert_err!(call, Error::<Test>::InvalidPairingProof);
    });
}

#[test]
fn test_update_processor_pairings_failure_2() {
    ExtBuilder::default().build().execute_with(|| {
        let (signer, processor_account) = generate_account();
        let _ = Timestamp::set(RuntimeOrigin::none(), 1657363915010);
        let timestamp = 1657363915002u128;
        let signature_1 = generate_signature(&signer, &alice_account_id(), timestamp, 1);
        let signature_2 = generate_signature(&signer, &alice_account_id(), timestamp, 2);
        let updates = vec![
            ProcessorPairingUpdateFor::<Test> {
                operation: ListUpdateOperation::Add,
                item: ProcessorPairingFor::<Test>::new_with_proof(
                    processor_account.clone(),
                    timestamp,
                    signature_1,
                ),
            },
            ProcessorPairingUpdateFor::<Test> {
                operation: ListUpdateOperation::Add,
                item: ProcessorPairingFor::<Test>::new_with_proof(
                    processor_account.clone(),
                    timestamp,
                    signature_2,
                ),
            },
        ];
        let call = AcurastProcessorManager::update_processor_pairings(
            RuntimeOrigin::signed(alice_account_id()),
            updates.clone(),
        );
        assert_err!(call, Error::<Test>::ProcessorAlreadyPaired);
    });
}

#[test]
fn test_update_processor_pairings_failure_3() {
    ExtBuilder::default().build().execute_with(|| {
        let (signer, processor_account) = generate_account();
        let _ = Timestamp::set(RuntimeOrigin::none(), 1657363915010);
        let timestamp = 1657363915002u128;
        let signature_1 = generate_signature(&signer, &alice_account_id(), timestamp, 1);
        let signature_2 = generate_signature(&signer, &bob_account_id(), timestamp, 1);
        let updates = vec![ProcessorPairingUpdateFor::<Test> {
            operation: ListUpdateOperation::Add,
            item: ProcessorPairingFor::<Test>::new_with_proof(
                processor_account.clone(),
                timestamp,
                signature_1,
            ),
        }];
        let call = AcurastProcessorManager::update_processor_pairings(
            RuntimeOrigin::signed(alice_account_id()),
            updates.clone(),
        );
        assert_ok!(call);

        let updates = vec![ProcessorPairingUpdateFor::<Test> {
            operation: ListUpdateOperation::Add,
            item: ProcessorPairingFor::<Test>::new_with_proof(
                processor_account.clone(),
                timestamp,
                signature_2,
            ),
        }];
        let call = AcurastProcessorManager::update_processor_pairings(
            RuntimeOrigin::signed(bob_account_id()),
            updates.clone(),
        );
        assert_err!(call, Error::<Test>::ProcessorPairedWithAnotherManager);
    });
}

#[test]
fn test_update_processor_pairings_failure_4() {
    ExtBuilder::default().build().execute_with(|| {
        let (signer, processor_account) = generate_account();
        let _ = Timestamp::set(RuntimeOrigin::none(), 1657363915010);
        let timestamp = 1657363915002u128;
        let signature = generate_signature(&signer, &alice_account_id(), timestamp, 1);
        let updates = vec![
            ProcessorPairingUpdateFor::<Test> {
                operation: ListUpdateOperation::Add,
                item: ProcessorPairingFor::<Test>::new_with_proof(
                    processor_account.clone(),
                    timestamp,
                    signature.clone(),
                ),
            },
            ProcessorPairingUpdateFor::<Test> {
                operation: ListUpdateOperation::Add,
                item: ProcessorPairingFor::<Test>::new_with_proof(
                    processor_account.clone(),
                    timestamp,
                    signature.clone(),
                ),
            },
            ProcessorPairingUpdateFor::<Test> {
                operation: ListUpdateOperation::Add,
                item: ProcessorPairingFor::<Test>::new_with_proof(
                    processor_account.clone(),
                    timestamp,
                    signature.clone(),
                ),
            },
            ProcessorPairingUpdateFor::<Test> {
                operation: ListUpdateOperation::Add,
                item: ProcessorPairingFor::<Test>::new_with_proof(
                    processor_account.clone(),
                    timestamp,
                    signature.clone(),
                ),
            },
            ProcessorPairingUpdateFor::<Test> {
                operation: ListUpdateOperation::Add,
                item: ProcessorPairingFor::<Test>::new_with_proof(
                    processor_account.clone(),
                    timestamp,
                    signature.clone(),
                ),
            },
            ProcessorPairingUpdateFor::<Test> {
                operation: ListUpdateOperation::Add,
                item: ProcessorPairingFor::<Test>::new_with_proof(
                    processor_account.clone(),
                    timestamp,
                    signature,
                ),
            },
        ];
        let call = AcurastProcessorManager::update_processor_pairings(
            RuntimeOrigin::signed(alice_account_id()),
            updates.clone(),
        );
        assert_err!(call, Error::<Test>::TooManyPairingUpdates);
    });
}

#[test]
fn test_recover_funds_succeed_1() {
    ExtBuilder::default().build().execute_with(|| {
        let (signer, processor_account) = generate_account();
        let _ = Timestamp::set(RuntimeOrigin::none(), 1657363915010);
        let timestamp = 1657363915002u128;
        let signature = generate_signature(&signer, &alice_account_id(), timestamp, 1);
        let updates = vec![ProcessorPairingUpdateFor::<Test> {
            operation: ListUpdateOperation::Add,
            item: ProcessorPairingFor::<Test>::new_with_proof(
                processor_account.clone(),
                timestamp,
                signature.clone(),
            ),
        }];
        assert_ok!(AcurastProcessorManager::update_processor_pairings(
            RuntimeOrigin::signed(alice_account_id()),
            updates.clone(),
        ));
        assert_ok!(Balances::transfer(
            RuntimeOrigin::signed(alice_account_id()),
            processor_account.clone().into(),
            10_000_000
        ));
        assert_eq!(Balances::balance(&alice_account_id()), 90_000_000);
        assert_ok!(Assets::transfer(
            RuntimeOrigin::signed(alice_account_id()),
            codec::Compact(22),
            processor_account.clone().into(),
            1_000_000
        ));
        assert_eq!(1_000_000, Assets::balance(22, &processor_account));

        let call = AcurastProcessorManager::recover_funds(
            RuntimeOrigin::signed(alice_account_id()),
            processor_account.clone().into(),
            alice_account_id().into(),
        );

        assert_ok!(call);
        assert_eq!(Balances::balance(&alice_account_id()), 99_999_000); // 1_000 of existensial balance remains on the processor
        assert_eq!(Assets::balance(22, &processor_account), 0);

        assert_eq!(
            events().last().unwrap(),
            &RuntimeEvent::AcurastProcessorManager(Event::ProcessorFundsRecovered(
                processor_account,
                alice_account_id()
            )),
        );
    });
}

#[test]
fn test_recover_funds_succeed_2() {
    ExtBuilder::default().build().execute_with(|| {
        let (signer, processor_account) = generate_account();
        let _ = Timestamp::set(RuntimeOrigin::none(), 1657363915010);
        let timestamp = 1657363915002u128;
        let signature = generate_signature(&signer, &alice_account_id(), timestamp, 1);
        let updates = vec![ProcessorPairingUpdateFor::<Test> {
            operation: ListUpdateOperation::Add,
            item: ProcessorPairingFor::<Test>::new_with_proof(
                processor_account.clone(),
                timestamp,
                signature.clone(),
            ),
        }];
        assert_ok!(AcurastProcessorManager::update_processor_pairings(
            RuntimeOrigin::signed(alice_account_id()),
            updates.clone(),
        ));

        let call = AcurastProcessorManager::recover_funds(
            RuntimeOrigin::signed(alice_account_id()),
            processor_account.clone().into(),
            alice_account_id().into(),
        );

        assert_ok!(call);

        assert_eq!(
            events().last().unwrap(),
            &RuntimeEvent::AcurastProcessorManager(Event::ProcessorFundsRecovered(
                processor_account,
                alice_account_id()
            )),
        );
    });
}

#[test]
fn test_recover_funds_failure_1() {
    ExtBuilder::default().build().execute_with(|| {
        let (signer, processor_account) = generate_account();
        let _ = Timestamp::set(RuntimeOrigin::none(), 1657363915010);
        let timestamp = 1657363915002u128;
        let signature = generate_signature(&signer, &alice_account_id(), timestamp, 1);
        let updates = vec![ProcessorPairingUpdateFor::<Test> {
            operation: ListUpdateOperation::Add,
            item: ProcessorPairingFor::<Test>::new_with_proof(
                processor_account.clone(),
                timestamp,
                signature.clone(),
            ),
        }];
        assert_ok!(AcurastProcessorManager::update_processor_pairings(
            RuntimeOrigin::signed(alice_account_id()),
            updates.clone(),
        ));

        let (_, processor_account) = generate_account();

        let call = AcurastProcessorManager::recover_funds(
            RuntimeOrigin::signed(alice_account_id()),
            processor_account.clone().into(),
            alice_account_id().into(),
        );

        assert_err!(call, Error::<Test>::ProcessorHasNoManager);
    });
}

#[test]
fn test_recover_funds_failure_2() {
    ExtBuilder::default().build().execute_with(|| {
        let (signer, processor_account) = generate_account();
        let _ = Timestamp::set(RuntimeOrigin::none(), 1657363915010);
        let timestamp = 1657363915002u128;
        let signature = generate_signature(&signer, &alice_account_id(), timestamp, 1);
        let updates = vec![ProcessorPairingUpdateFor::<Test> {
            operation: ListUpdateOperation::Add,
            item: ProcessorPairingFor::<Test>::new_with_proof(
                processor_account.clone(),
                timestamp,
                signature.clone(),
            ),
        }];
        assert_ok!(AcurastProcessorManager::update_processor_pairings(
            RuntimeOrigin::signed(alice_account_id()),
            updates.clone(),
        ));

        assert_ok!(AcurastProcessorManager::update_processor_pairings(
            RuntimeOrigin::signed(bob_account_id()),
            vec![],
        ));

        let call = AcurastProcessorManager::recover_funds(
            RuntimeOrigin::signed(bob_account_id()),
            processor_account.clone().into(),
            alice_account_id().into(),
        );

        assert_err!(call, Error::<Test>::ProcessorPairedWithAnotherManager);
    });
}

#[test]
fn test_pair_with_manager() {
    ExtBuilder::default().build().execute_with(|| {
        let (signer, manager_account) = generate_account();
        let (_, processor_account) = generate_account();
        let _ = Timestamp::set(RuntimeOrigin::none(), 1657363915010);
        let timestamp = 1657363915002u128;
        let signature = generate_signature(&signer, &manager_account, timestamp, 1);
        let update = ProcessorPairingFor::<Test>::new_with_proof(
            manager_account.clone(),
            timestamp,
            signature,
        );
        assert_ok!(AcurastProcessorManager::pair_with_manager(
            RuntimeOrigin::signed(processor_account.clone()),
            update.clone(),
        ));

        assert_eq!(Some(1), AcurastProcessorManager::last_manager_id());
        assert_eq!(
            Some(1),
            AcurastProcessorManager::manager_id_for_processor(&processor_account)
        );
        assert_eq!(
            Some(manager_account.clone()),
            AcurastProcessorManager::manager_for_processor(&processor_account)
        );
        assert!(AcurastProcessorManager::managed_processors(1, &processor_account).is_some());
        let last_events = events();
        assert_eq!(
            last_events[(last_events.len() - 2)..],
            vec![
                RuntimeEvent::AcurastProcessorManager(Event::ManagerCreated(manager_account, 1)),
                RuntimeEvent::AcurastProcessorManager(Event::ProcessorPaired(
                    processor_account,
                    update
                )),
            ]
        );
    });
}
