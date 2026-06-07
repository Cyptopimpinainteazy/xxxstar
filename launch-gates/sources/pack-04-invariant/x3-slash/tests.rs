//! Tests for the x3-slash pallet.

#[cfg(test)]
mod tests {
    use crate::mock::*;
    use crate::*;
    use frame_support::assert_ok;
    use sp_runtime::AccountId32;

    #[test]
    fn test_post_bond() {
        new_test_ext().execute_with(|| {
            let agent = AccountId32::from([0u8; 32]);
            let amount = 5_000_000u128;

            assert_ok!(Slash::post_bond(
                RuntimeOrigin::signed(agent.clone()),
                amount,
                None
            ));

            // Verify bond was stored
            let bonds = crate::BondsByAgent::<Test>::get(&agent);
            assert_eq!(bonds.len(), 1);
        });
    }

    #[test]
    fn test_release_bond() {
        new_test_ext().execute_with(|| {
            let agent = AccountId32::from([0u8; 32]);
            let amount = 5_000_000u128;

            // Post bond
            assert_ok!(Slash::post_bond(
                RuntimeOrigin::signed(agent.clone()),
                amount,
                None
            ));

            let bonds = crate::BondsByAgent::<Test>::get(&agent);
            let bond_id = bonds[0];

            // Release bond
            assert_ok!(Slash::release_bond(RuntimeOrigin::root(), bond_id));

            // Verify bond status changed
            let bond = Bonds::<Test>::get(bond_id).unwrap();
            assert_eq!(bond.status, crate::BondStatus::Released);
        });
    }

    #[test]
    fn test_slash_bond() {
        new_test_ext().execute_with(|| {
            let agent = AccountId32::from([0u8; 32]);
            let amount = 5_000_000u128;

            // Post bond
            assert_ok!(Slash::post_bond(
                RuntimeOrigin::signed(agent.clone()),
                amount,
                None
            ));

            let bonds = crate::BondsByAgent::<Test>::get(&agent);
            let bond_id = bonds[0];

            // Slash bond (Major severity = 2)
            assert_ok!(Slash::slash_bond(
                RuntimeOrigin::root(),
                bond_id,
                2,
                vec![1u8; 64]
            ));

            // Verify bond status changed
            let bond = Bonds::<Test>::get(bond_id).unwrap();
            assert_eq!(bond.status, crate::BondStatus::FullySlashed);

            // Verify slash record was created
            let slash = SlashRecords::<Test>::get(0).unwrap();
            assert_eq!(slash.bond_id, bond_id);
            assert_eq!(slash.severity, 2);
        });
    }
}
