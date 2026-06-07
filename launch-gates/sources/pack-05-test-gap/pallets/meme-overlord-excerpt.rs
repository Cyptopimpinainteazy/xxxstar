#![deny(unsafe_code)]
//! # Meme Overlord Pallet
//!
//! Auto-generates celebratory memes for profitable trades on X3 Chain.
//!
//! ## Overview
//!
//! The Meme Overlord Pallet automatically tracks profitable trades and generates
//! meme content based on trade performance, creating a fun social layer for DeFi.
//!
//! ## Features
//!
//! - **Trade Achievement System**: Track milestones and achievements
//! - **Meme Template Registry**: On-chain meme template storage
//! - **Auto-generation**: Generate memes based on trade results
//! - **Social Rewards**: Earn MEME tokens for viral trades
//! - **Leaderboard**: Track top traders and their meme collections
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     MEME OVERLORD PALLET                        │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
//! │  │ Trade Listener  │──│  Achievement    │──│  Meme Generator │ │
//! │  │                 │  │  Tracker        │  │                 │ │
//! │  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
//! │           │                    │                    │          │
//! │           ▼                    ▼                    ▼          │
//! │  ┌─────────────────────────────────────────────────────────┐   │
//! │  │                    Storage Layer                        │   │
//! │  │  Templates | Achievements | Memes | Leaderboard        │   │
//! │  └─────────────────────────────────────────────────────────┘   │
//! │           │                    │                    │          │
//! │           ▼                    ▼                    ▼          │
//! │  ┌─────────────────────────────────────────────────────────┐   │
//! │  │                    Event Emission                       │   │
//! │  │  MemeGenerated | AchievementUnlocked | Viral            │   │
//! │  └─────────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use crate::weights::WeightInfo;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::SaturatedConversion;
    use sp_std::prelude::*;

    /// Pallet configuration trait
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics
        type WeightInfo: WeightInfo;

        /// Maximum template name length
        #[pallet::constant]
        type MaxTemplateNameLength: Get<u32>;

        /// Maximum meme data length (IPFS hash or data URL)
        #[pallet::constant]
        type MaxMemeDataLength: Get<u32>;

        /// Maximum achievements per account
        #[pallet::constant]
        type MaxAchievements: Get<u32>;

        /// Minimum profit for meme generation (in basis points)
        #[pallet::constant]
        type MinProfitBps: Get<u32>;

        /// Viral threshold (likes/shares needed)
        #[pallet::constant]
        type ViralThreshold: Get<u32>;
    }

    use frame_support::traits::StorageVersion;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    // ========== STORAGE ==========

    /// Registered meme templates
    #[pallet::storage]
    #[pallet::getter(fn templates)]
    pub type Templates<T: Config> =
        StorageMap<_, Blake2_128Concat, TemplateId, MemeTemplate<T>, OptionQuery>;

    /// Next template ID
    #[pallet::storage]
    #[pallet::getter(fn next_template_id)]
    pub type NextTemplateId<T: Config> = StorageValue<_, TemplateId, ValueQuery>;

    /// Generated memes
    #[pallet::storage]
    #[pallet::getter(fn memes)]
    pub type Memes<T: Config> = StorageMap<_, Blake2_128Concat, MemeId, Meme<T>, OptionQuery>;

    /// Next meme ID
    #[pallet::storage]
    #[pallet::getter(fn next_meme_id)]
    pub type NextMemeId<T: Config> = StorageValue<_, MemeId, ValueQuery>;

    /// User achievements
    #[pallet::storage]
    #[pallet::getter(fn achievements)]
    pub type Achievements<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<Achievement, T::MaxAchievements>,
        ValueQuery,
    >;

    /// Trade statistics per account
    #[pallet::storage]
    #[pallet::getter(fn trader_stats)]
    pub type TraderStats<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, TradeStats, ValueQuery>;

    /// Leaderboard (top traders by profit)
    #[pallet::storage]
    #[pallet::getter(fn leaderboard)]
    pub type Leaderboard<T: Config> =
        StorageValue<_, BoundedVec<(T::AccountId, u128), ConstU32<100>>, ValueQuery>;

    /// Meme social engagement (likes, shares)
    #[pallet::storage]
    #[pallet::getter(fn meme_engagement)]
    pub type MemeEngagement<T: Config> =
        StorageMap<_, Blake2_128Concat, MemeId, Engagement, ValueQuery>;

    /// Viral memes
    #[pallet::storage]
    #[pallet::getter(fn viral_memes)]
    pub type ViralMemes<T: Config> = StorageValue<_, BoundedVec<MemeId, ConstU32<50>>, ValueQuery>;

    // ========== TYPES ==========

    /// Template ID type
    pub type TemplateId = u64;

    /// Meme ID type
    pub type MemeId = u64;

    /// Meme template structure
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq)]
    #[scale_info(skip_type_params(T))]
    pub struct MemeTemplate<T: Config> {
        /// Template ID
        pub id: TemplateId,
        /// Template name
        pub name: BoundedVec<u8, T::MaxTemplateNameLength>,
        /// Template category
        pub category: MemeCategory,
        /// Base image IPFS hash
        pub image_cid: BoundedVec<u8, T::MaxMemeDataLength>,
        /// Text positions for overlay
        pub text_positions: TextPositions,
        /// Creator account
        pub creator: T::AccountId,
        /// Usage count
        pub usage_count: u32,
        /// Is active
        pub is_active: bool,
    }

    /// Generated meme
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq)]
    #[scale_info(skip_type_params(T))]
    pub struct Meme<T: Config> {
        /// Meme ID
        pub id: MemeId,
        /// Template used
        pub template_id: TemplateId,
        /// Generated image CID
        pub image_cid: BoundedVec<u8, T::MaxMemeDataLength>,
        /// Trader account
        pub trader: T::AccountId,
        /// Trade that triggered this meme
        pub trade_hash: [u8; 32],
        /// Profit in basis points
        pub profit_bps: u32,
        /// Achievement unlocked
        pub achievement: Option<AchievementType>,
        /// Generated at block
        pub generated_at: BlockNumberFor<T>,
        /// Is viral
        pub is_viral: bool,
    }

    /// Meme categories
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Copy)]
    pub enum MemeCategory {
        /// Small profit memes (1-10%)
        SmallGain,
        /// Medium profit memes (10-50%)
        MediumGain,
        /// Large profit memes (50-100%)
        LargeGain,
        /// Massive profit memes (100%+)
        MassiveGain,
        /// Loss consolation memes
        Loss,
        /// Milestone achievement memes
        Milestone,
        /// Streak memes (consecutive wins)
        Streak,
        /// Diamond hands memes (holding through volatility)
        DiamondHands,
    }

    /// Text positions for meme overlay
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Default)]
    pub struct TextPositions {
        pub top_x: u16,
        pub top_y: u16,
        pub bottom_x: u16,
        pub bottom_y: u16,
    }

    /// Achievement types
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Copy)]
    pub enum AchievementType {
        /// First trade
        FirstTrade,
        /// First profit
        FirstProfit,
        /// 10x profit
        TenBagger,
        /// 100x profit
        HundredBagger,
        /// 1000x profit (legendary)
        ThousandBagger,
        /// 10 consecutive wins
        WinStreak10,
        /// 50 consecutive wins
        WinStreak50,
        /// 100 consecutive wins
        WinStreak100,
        /// First viral meme
        FirstViral,
        /// 10 viral memes
        MemeKing,
        /// Held through 50% drop and recovered
        DiamondHands,
        /// First cross-chain trade
        BridgeBuilder,
        /// Million dollar trade
        Millionaire,
        /// Trade during high volatility
        VolatilityMaster,
        /// Early adopter
        OG,
    }

    /// Achievement record
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq)]
    pub struct Achievement {
        pub achievement_type: AchievementType,
        pub unlocked_at: u64,
        pub trade_hash: Option<[u8; 32]>,
    }

    /// Trade statistics
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Default)]
    pub struct TradeStats {
        pub total_trades: u64,
        pub profitable_trades: u64,
        pub total_profit: u128,
        pub total_loss: u128,
        pub current_streak: u32,
        pub best_streak: u32,
        pub memes_generated: u32,
        pub viral_memes: u32,
    }

    /// Social engagement for a meme
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Default)]
    pub struct Engagement {
        pub likes: u32,
        pub shares: u32,
        pub comments: u32,
    }

    // ========== EVENTS ==========

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new meme template was registered
        TemplateRegistered {
            template_id: TemplateId,
            creator: T::AccountId,
            category: MemeCategory,
        },
        /// A meme was generated for a trade
        MemeGenerated {
            meme_id: MemeId,
            trader: T::AccountId,
            template_id: TemplateId,
            profit_bps: u32,
        },
        /// An achievement was unlocked
        AchievementUnlocked {
            trader: T::AccountId,
            achievement: AchievementType,
        },
        /// A meme went viral
        MemeWentViral {
            meme_id: MemeId,
            trader: T::AccountId,
            likes: u32,
            shares: u32,
        },
        /// Leaderboard was updated
        LeaderboardUpdated {
            trader: T::AccountId,
            new_rank: u32,
            total_profit: u128,
        },
        /// Meme was liked
        MemeLiked {
            meme_id: MemeId,
            liker: T::AccountId,
        },
        /// Meme was shared
        MemeShared {
            meme_id: MemeId,
            sharer: T::AccountId,
        },
    }

    // ========== ERRORS ==========

    #[pallet::error]
    pub enum Error<T> {
        /// Template not found
        TemplateNotFound,
        /// Template name too long
        TemplateNameTooLong,
        /// Meme data too long
        MemeDataTooLong,
        /// Meme not found
        MemeNotFound,
        /// Maximum achievements reached
        MaxAchievementsReached,
        /// Already liked this meme
        AlreadyLiked,
        /// Profit too low for meme generation
        ProfitTooLow,
        /// Template is inactive
        TemplateInactive,
        /// Not authorized
        NotAuthorized,
    }

    // ========== EXTRINSICS ==========

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new meme template
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_template())]
        pub fn register_template(
            origin: OriginFor<T>,
            name: Vec<u8>,
            category: MemeCategory,
            image_cid: Vec<u8>,
            text_positions: TextPositions,
        ) -> DispatchResult {
            let creator = ensure_signed(origin)?;

            let bounded_name: BoundedVec<u8, T::MaxTemplateNameLength> = name
                .try_into()
                .map_err(|_| Error::<T>::TemplateNameTooLong)?;

            let bounded_cid: BoundedVec<u8, T::MaxMemeDataLength> = image_cid
                .try_into()
                .map_err(|_| Error::<T>::MemeDataTooLong)?;

            let template_id = NextTemplateId::<T>::get();
            NextTemplateId::<T>::put(template_id + 1);

            let template = MemeTemplate {
                id: template_id,
                name: bounded_name,
                category,
                image_cid: bounded_cid,
                text_positions,
                creator: creator.clone(),
                usage_count: 0,
                is_active: true,
            };

            Templates::<T>::insert(template_id, template);

            Self::deposit_event(Event::TemplateRegistered {
                template_id,
                creator,
                category,
            });

            Ok(())
        }

        /// Generate meme for a trade (called by x3-kernel after profitable trade)
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::generate_meme())]
        pub fn generate_meme(
            origin: OriginFor<T>,
            trade_hash: [u8; 32],
            profit_bps: u32,
        ) -> DispatchResult {
            let trader = ensure_signed(origin)?;

            // Check minimum profit
            ensure!(
                profit_bps >= T::MinProfitBps::get(),
                Error::<T>::ProfitTooLow
            );

            // Select template based on profit
            let category = Self::category_for_profit(profit_bps);
            let template_id = Self::select_template(category)?;

            // Generate meme
            let meme_id = Self::do_generate_meme(&trader, template_id, trade_hash, profit_bps)?;

            // Update stats
            Self::update_trader_stats(&trader, profit_bps);

            // Check achievements
            Self::check_achievements(&trader, profit_bps)?;

            Self::deposit_event(Event::MemeGenerated {
                meme_id,
                trader,
                template_id,
                profit_bps,
            });

            Ok(())
        }

        /// Like a meme
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::like_meme())]
        pub fn like_meme(origin: OriginFor<T>, meme_id: MemeId) -> DispatchResult {
            let liker = ensure_signed(origin)?;

            ensure!(Memes::<T>::contains_key(meme_id), Error::<T>::MemeNotFound);

            MemeEngagement::<T>::mutate(meme_id, |engagement| {
                engagement.likes += 1;
            });

            // Check if went viral
            Self::check_viral_status(meme_id)?;

            Self::deposit_event(Event::MemeLiked { meme_id, liker });

            Ok(())
        }

        /// Share a meme
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::share_meme())]
        pub fn share_meme(origin: OriginFor<T>, meme_id: MemeId) -> DispatchResult {
            let sharer = ensure_signed(origin)?;

            ensure!(Memes::<T>::contains_key(meme_id), Error::<T>::MemeNotFound);

            MemeEngagement::<T>::mutate(meme_id, |engagement| {
                engagement.shares += 1;
