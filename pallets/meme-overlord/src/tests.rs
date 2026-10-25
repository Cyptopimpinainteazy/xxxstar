use crate::{
    mock::*,
    pallet::Pallet,
    pallet::{
        AchievementType, Error, Event, MemeCategory, MemeEngagement, Memes, NextMemeId,
        NextTemplateId, Templates, TraderStats, ViralMemes,
    },
};
use frame_support::{assert_noop, assert_ok};

fn text_positions() -> crate::pallet::TextPositions {
    crate::pallet::TextPositions {
        top_x: 1,
        top_y: 2,
        bottom_x: 3,
        bottom_y: 4,
    }
}

fn register_template(creator: u64, category: MemeCategory) {
    assert_ok!(Pallet::<Test>::register_template(
        RuntimeOrigin::signed(creator),
        b"deterministic-template".to_vec(),
        category,
        b"ipfs://x3-template".to_vec(),
        text_positions(),
    ));
}

#[cfg(test)]
#[test]
fn register_template_assigns_ids_and_emits_event() {
    new_test_ext().execute_with(|| {
        register_template(1, MemeCategory::SmallGain);

        assert_eq!(NextTemplateId::<Test>::get(), 1);
        let template = Templates::<Test>::get(0).expect("template must be stored");
        assert_eq!(template.creator, 1);
        assert_eq!(template.category, MemeCategory::SmallGain);
        assert!(template.is_active);
        assert_eq!(template.usage_count, 0);

        System::assert_last_event(
            Event::TemplateRegistered {
                template_id: 0,
                creator: 1,
                category: MemeCategory::SmallGain,
            }
            .into(),
        );
    });
}

#[cfg(test)]
#[test]
fn generate_meme_rejects_profit_below_governance_minimum() {
    new_test_ext().execute_with(|| {
        register_template(1, MemeCategory::SmallGain);

        assert_noop!(
            Pallet::<Test>::generate_meme(RuntimeOrigin::signed(1), [7u8; 32], 99),
            Error::<Test>::ProfitTooLow
        );
        assert_eq!(NextMemeId::<Test>::get(), 0);
    });
}

#[cfg(test)]
#[test]
fn generate_meme_updates_real_state_and_unlocks_first_trade_achievements() {
    new_test_ext().execute_with(|| {
        register_template(1, MemeCategory::SmallGain);

        assert_ok!(Pallet::<Test>::generate_meme(
            RuntimeOrigin::signed(1),
            [9u8; 32],
            500
        ));

        let meme = Memes::<Test>::get(0).expect("generated meme must be stored");
        assert_eq!(meme.trader, 1);
        assert_eq!(meme.profit_bps, 500);
        assert_eq!(meme.generated_at, 1);
        assert_eq!(Templates::<Test>::get(0).unwrap().usage_count, 1);

        let stats = TraderStats::<Test>::get(1);
        assert_eq!(stats.total_trades, 1);
        assert_eq!(stats.profitable_trades, 1);
        assert_eq!(stats.total_profit, 500);
        assert_eq!(stats.current_streak, 1);
        assert_eq!(stats.best_streak, 1);
        assert_eq!(stats.memes_generated, 1);

        let achievements = crate::pallet::Achievements::<Test>::get(1);
        assert!(achievements
            .iter()
            .any(|a| a.achievement_type == AchievementType::FirstTrade));
        assert!(achievements
            .iter()
            .any(|a| a.achievement_type == AchievementType::FirstProfit));
    });
}

#[cfg(test)]
#[test]
fn duplicate_like_is_rejected_without_changing_engagement() {
    new_test_ext().execute_with(|| {
        register_template(1, MemeCategory::SmallGain);
        assert_ok!(Pallet::<Test>::generate_meme(
            RuntimeOrigin::signed(1),
            [3u8; 32],
            500
        ));

        assert_ok!(Pallet::<Test>::like_meme(RuntimeOrigin::signed(2), 0));
        assert_eq!(MemeEngagement::<Test>::get(0).likes, 1);

        assert_noop!(
            Pallet::<Test>::like_meme(RuntimeOrigin::signed(2), 0),
            Error::<Test>::AlreadyLiked
        );
        assert_eq!(MemeEngagement::<Test>::get(0).likes, 1);
    });
}

#[cfg(test)]
#[test]
fn distinct_likes_trigger_viral_state_and_achievement_once() {
    new_test_ext().execute_with(|| {
        register_template(1, MemeCategory::SmallGain);
        assert_ok!(Pallet::<Test>::generate_meme(
            RuntimeOrigin::signed(1),
            [4u8; 32],
            500
        ));

        assert_ok!(Pallet::<Test>::like_meme(RuntimeOrigin::signed(2), 0));
        assert_ok!(Pallet::<Test>::like_meme(RuntimeOrigin::signed(3), 0));

        assert!(Memes::<Test>::get(0).unwrap().is_viral);
        assert_eq!(ViralMemes::<Test>::get().to_vec(), vec![0]);
        assert_eq!(TraderStats::<Test>::get(1).viral_memes, 1);

        let achievements = crate::pallet::Achievements::<Test>::get(1);
        assert_eq!(
            achievements
                .iter()
                .filter(|a| a.achievement_type == AchievementType::FirstViral)
                .count(),
            1
        );
    });
}

#[cfg(test)]
#[test]
fn only_creator_can_deactivate_template() {
    new_test_ext().execute_with(|| {
        register_template(1, MemeCategory::SmallGain);

        assert_noop!(
            Pallet::<Test>::deactivate_template(RuntimeOrigin::signed(2), 0),
            Error::<Test>::NotAuthorized
        );
        assert_ok!(Pallet::<Test>::deactivate_template(
            RuntimeOrigin::signed(1),
            0
        ));
        assert!(!Templates::<Test>::get(0).unwrap().is_active);
    });
}

#[cfg(test)]
#[test]
fn inactive_only_template_cannot_generate_a_meme() {
    new_test_ext().execute_with(|| {
        register_template(1, MemeCategory::SmallGain);
        assert_ok!(Pallet::<Test>::deactivate_template(
            RuntimeOrigin::signed(1),
            0
        ));

        assert_noop!(
            Pallet::<Test>::generate_meme(RuntimeOrigin::signed(1), [5u8; 32], 500),
            Error::<Test>::TemplateNotFound
        );
    });
}
