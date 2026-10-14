// SPDX-License-Identifier: Apache-2.0
//
// pallet-x3-app-registry — the X3 Guardian application registry.
//
// This is the spine of Guardian. It stores, per application version, the exact
// artifacts that were evaluated and the certification state attached to them.
// Everything else (security gate, trust gate, runner, watchdog) reads or writes
// through it.
//
// Responsibilities:
//   * Application identity: id, owner, name, VM, category, addresses.
//   * Artifact binding: the bytecode / source / manifest hashes a certification
//     is bound to (spec §24 R2, §33). Certification is bound to an *artifact*,
//     never to a name, repo, or owner.
//   * Certification tiers (spec §3): EXPERIMENTAL, UNDER_REVIEW, X3_VERIFIED,
//     RESTRICTED.
//   * Restrictions / revocations with reason codes, keeping full history
//     (spec §24 R5, §47).
//   * Upgrade history: every version is retained; adding new bytecode can never
//     inherit the previous version's certification (spec §59 SP3).
//   * Address index (spec §50): one VM address resolves to exactly one app.
//
// Explicit non-responsibilities:
//   * Does not run analyses, fuzzing, or the exploit corpus (that is the runner).
//   * Does not define the Privilege Map (that is pallet-x3-trust-gate).
//   * Does not hold security policies or thresholds (that is
//     pallet-x3-security-gate).
//   * Does not move funds or hold balances.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

pub use pallet::*;

pub mod weights;
pub use weights::WeightInfo;

use frame_support::pallet_prelude::{
    Decode, DecodeWithMemTracking, Encode, MaxEncodedLen, RuntimeDebug, TypeInfo,
};
use sp_core::H256;

/// Sequential identifier for an application, assigned by the registry.
pub type ApplicationId = u64;

/// A normalized, fixed-width VM address. EVM (20-byte) and SVM (32-byte)
/// addresses must be normalized to 32 bytes by the caller before binding.
pub type AppAddress = (GuardianVm, [u8; 32]);

/// Execution domain an application targets (spec §5, §19).
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub enum GuardianVm {
    /// Ethereum-compatible execution domain.
    Evm,
    /// Solana-compatible execution domain.
    Svm,
    /// X3's native execution domain.
    X3Vm,
}

/// Certification tier (spec §3).
///
/// A tier is a claim about a *specific artifact hash*, not about a name. This
/// enum is the single source of truth for "how much do we trust this app".
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub enum CertificationTier {
    /// Permissionless, unendorsed (T1).
    Experimental,
    /// Submitted and awaiting analysis (T2).
    UnderReview,
    /// Passed the mandatory gate for its artifact hash (T3).
    X3Verified,
    /// Privileges removed, history kept (T4).
    Restricted,
}

impl CertificationTier {
    /// Whether an application at this tier may use Guardian-controlled
    /// privileges. Anything other than a live `X3Verified` tier fails closed.
    pub fn grants_privileges(&self) -> bool {
        matches!(self, CertificationTier::X3Verified)
    }
}

/// Machine-readable reason for a restriction or revocation (spec §24 R5, §47).
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub enum RestrictionReason {
    /// A security finding was confirmed.
    SecurityFinding,
    /// Automated revalidation against a new epoch failed.
    FailedRevalidation,
    /// The owner asked to withdraw the application.
    OwnerRequest,
    /// The certified artifact no longer matches the deployed bytecode.
    ArtifactMismatch,
    /// A Guardian policy was violated.
    PolicyViolation,
    /// An incident-response action was taken.
    Incident,
    /// The manifest was superseded by a newer one (spec §47: no deletion).
    ManifestSuperseded,
}

/// The exact artifact hashes a certification is bound to (spec §24 R2, §33).
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub struct ArtifactHashes {
    /// Hash of the deployed bytecode / program.
    pub bytecode: H256,
    /// Hash of the human-readable source that produced it.
    pub source: H256,
    /// Hash of the signed Security Manifest (spec §23).
    pub manifest: H256,
}

/// Versioned standards an artifact was evaluated against (spec §24 R3, §28).
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub struct StandardRefs {
    /// Guardian standard version under which the artifact was evaluated.
    pub guardian_standard: u16,
    /// Trust standard version under which the artifact was evaluated.
    pub trust_standard: u16,
    /// Exploit-corpus version under which the artifact was evaluated.
    pub exploit_corpus: u16,
}

/// Read-only view of the registry for the security gate, trust gate, wallet,
/// explorer, and Agent Law. Kept separate from the concrete pallet so consumers
/// do not need tight pallet coupling.
pub trait ApplicationRegistryInspect<AccountId> {
    /// The current tier of an application, if it exists.
    fn tier(app_id: ApplicationId) -> Option<CertificationTier>;
    /// The owner of an application, if it exists.
    fn owner(app_id: ApplicationId) -> Option<AccountId>;
    /// The current (active) version index of an application.
    fn current_version(app_id: ApplicationId) -> Option<u32>;
    /// The hashes bound to the current version of an application.
    fn current_hashes(app_id: ApplicationId) -> Option<ArtifactHashes>;
    /// Whether an application is currently restricted or revoked (spec §59 SP4).
    ///
    /// Returns `None` when no such application is registered. `None` is *not*
    /// the same as unrestricted: an unknown application must never be read as
    /// permission. Callers making an access decision must prefer
    /// [`Self::has_guardian_privileges`], which is fail-closed and returns
    /// `false` for unknown, revoked and restricted applications alike; callers
    /// that surface status to a user (wallet/explorer, spec §45/§46) must show
    /// `None` as "unknown", never as "not restricted".
    fn is_restricted(app_id: ApplicationId) -> Option<bool>;
    /// Whether `bytecode` is exactly the artifact the current version is
    /// certified at, while that certification is live (spec §59 SP2/SP3).
    fn is_certified_artifact(app_id: ApplicationId, bytecode: H256) -> bool;
    /// Whether the application may currently use Guardian-controlled privileges.
    fn has_guardian_privileges(app_id: ApplicationId) -> bool;
    /// Whether a bytecode hash has been revoked (spec §59 SP4).
    fn is_artifact_revoked(bytecode: H256) -> bool;
    /// Canonical hash of the application's registered Security Manifest, if any
    /// (spec §23 M2/M3). Defaults to `None` so external implementers are not
    /// forced to provide it.
    fn manifest_hash(_app_id: ApplicationId) -> Option<H256> {
        None
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::{
        AppAddress, ApplicationId, ArtifactHashes, CertificationTier, GuardianVm,
        RestrictionReason, StandardRefs,
    };
    use crate::weights::WeightInfo;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_std::vec::Vec;

    /// Max length of the human-readable application name.
    pub type MaxNameLen = ConstU32<64>;
    /// Bounded application name.
    pub type BoundedName = BoundedVec<u8, MaxNameLen>;
    /// Max length of the human-readable version string in a manifest.
    pub type MaxVersionLen = ConstU32<32>;
    /// Max number of declared capability ids in a manifest.
    pub type MaxCapabilities = ConstU32<32>;
    /// Max number of off-chain evidence references in a manifest.
    pub type MaxEvidence = ConstU32<16>;

    /// A verifiable reference to off-chain evidence (spec §23 M4, §55). The chain
    /// stores the hash of the evidence, never the evidence itself.
    #[derive(
        Clone,
        Copy,
        PartialEq,
        Eq,
        Encode,
        Decode,
        DecodeWithMemTracking,
        MaxEncodedLen,
        TypeInfo,
        RuntimeDebug,
    )]
    pub struct EvidenceRef {
        /// Evidence kind (e.g. 0 = audit report, 1 = test transcript).
        pub kind: u8,
        /// Hash of the off-chain evidence document.
        pub uri_hash: H256,
    }

    /// The machine-readable Security Manifest (spec §23 M1): what an application
    /// claims about itself, bound to the exact artifact hashes it describes.
    #[derive(
        Clone,
        PartialEq,
        Eq,
        Encode,
        Decode,
        DecodeWithMemTracking,
        MaxEncodedLen,
        TypeInfo,
        RuntimeDebug,
    )]
    pub struct SecurityManifest {
        /// Application name.
        pub name: BoundedName,
        /// Human-readable version string.
        pub version: BoundedVec<u8, MaxVersionLen>,
        /// Execution domain.
        pub vm: GuardianVm,
        /// Application category id.
        pub category: u16,
        /// The exact artifact hashes this manifest describes.
        pub hashes: ArtifactHashes,
        /// The standards this manifest was authored against.
        pub standards: StandardRefs,
        /// Declared capability ids (interpreted by the trust gate).
        pub declared_capabilities: BoundedVec<u32, MaxCapabilities>,
        /// Off-chain evidence references.
        pub evidence: BoundedVec<EvidenceRef, MaxEvidence>,
    }

    impl SecurityManifest {
        /// Deterministic, field-sensitive hash over the canonical SCALE encoding
        /// (spec §23 M2). Field order is part of the wire format: reordering or
        /// changing any field changes the hash, so a recorded manifest hash
        /// cannot be silently reused for different content (spec §59 SP5).
        pub fn canonical_hash(&self) -> H256 {
            H256(sp_io::hashing::blake2_256(&self.encode()))
        }
    }

    /// Full on-chain application record.
    #[derive(
        Clone,
        PartialEq,
        Eq,
        Encode,
        Decode,
        DecodeWithMemTracking,
        MaxEncodedLen,
        TypeInfo,
        RuntimeDebug,
    )]
    #[scale_info(skip_type_params(T))]
    pub struct ApplicationRecord<T: Config> {
        /// Registry id.
        pub id: ApplicationId,
        /// Current owner (may differ from the original registrant).
        pub owner: T::AccountId,
        /// Human-readable name.
        pub name: BoundedName,
        /// Execution domain.
        pub vm: GuardianVm,
        /// Application category id (spec §34). Interpreted by the security gate.
        pub category: u16,
        /// Index of the current version in `Versions`.
        pub current_version: u32,
        /// Tier of the current version.
        pub tier: CertificationTier,
        /// Restriction reason, if any.
        pub restriction: Option<RestrictionReason>,
        /// Whether the application has been permanently revoked.
        pub revoked: bool,
        /// Block at registration.
        pub registered_at: BlockNumberFor<T>,
        /// Block of the last mutation.
        pub updated_at: BlockNumberFor<T>,
        /// Number of non-initial versions added (upgrade history length - 1).
        pub upgrade_count: u32,
    }

    /// Per-version record. Every version ever added is retained.
    #[derive(
        Clone,
        PartialEq,
        Eq,
        Encode,
        Decode,
        DecodeWithMemTracking,
        MaxEncodedLen,
        TypeInfo,
        RuntimeDebug,
    )]
    #[scale_info(skip_type_params(T))]
    pub struct VersionRecord<T: Config> {
        /// Version index.
        pub version: u32,
        /// The artifact hashes for this version.
        pub hashes: ArtifactHashes,
        /// Standards this version was evaluated against.
        pub standards: StandardRefs,
        /// Certification tier of this specific version.
        pub tier: CertificationTier,
        /// Block at which the version was submitted.
        pub submitted_at: BlockNumberFor<T>,
        /// Block at which this version was certified (if ever).
        pub certified_at: Option<BlockNumberFor<T>>,
        /// Block at which recertification is next due (if scheduled).
        pub next_review: Option<BlockNumberFor<T>>,
        /// Restriction reason, if this version was restricted.
        pub restriction: Option<RestrictionReason>,
    }

    // ── Storage ────────────────────────────────────────────────────────────

    /// ApplicationId → application record.
    #[pallet::storage]
    #[pallet::getter(fn applications)]
    pub type Applications<T: Config> =
        StorageMap<_, Blake2_128Concat, ApplicationId, ApplicationRecord<T>>;

    /// (ApplicationId, version) → version record. Never pruned.
    #[pallet::storage]
    #[pallet::getter(fn versions)]
    pub type Versions<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ApplicationId,
        Blake2_128Concat,
        u32,
        VersionRecord<T>,
    >;

    /// Next application id to assign. Ids are assigned sequentially from 0.
    #[pallet::storage]
    #[pallet::getter(fn next_application_id)]
    pub type NextApplicationId<T: Config> = StorageValue<_, ApplicationId, ValueQuery>;

    /// (VM, normalized address) → owning ApplicationId. One address, one app.
    #[pallet::storage]
    #[pallet::getter(fn address_owners)]
    pub type AddressOwners<T: Config> = StorageMap<_, Blake2_128Concat, AppAddress, ApplicationId>;

    /// ApplicationId → its bound addresses.
    #[pallet::storage]
    #[pallet::getter(fn app_addresses)]
    pub type AppAddresses<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        ApplicationId,
        BoundedVec<AppAddress, T::MaxAddressesPerApp>,
    >;

    /// Revoked bytecode hash → block at which it was revoked (spec §59 SP4).
    #[pallet::storage]
    #[pallet::getter(fn revoked_artifacts)]
    pub type RevokedArtifacts<T: Config> = StorageMap<_, Blake2_128Concat, H256, BlockNumberFor<T>>;

    /// ApplicationId → its currently registered Security Manifest.
    #[pallet::storage]
    #[pallet::getter(fn manifest_of)]
    pub type Manifests<T: Config> =
        StorageMap<_, Blake2_128Concat, ApplicationId, SecurityManifest>;

    /// ApplicationId → canonical hash of its registered manifest.
    #[pallet::storage]
    #[pallet::getter(fn manifest_hash_of)]
    pub type ManifestHashes<T: Config> = StorageMap<_, Blake2_128Concat, ApplicationId, H256>;

    // ── Config ─────────────────────────────────────────────────────────────

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Origin permitted to create and mutate an application. Must yield the
        /// owner account (typically a signed origin).
        type OwnerOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;
        /// Origin permitted to certify, restrict, and revoke applications.
        /// MUST be a privileged origin (Root or a governance council), never
        /// `EnsureSigned` — certification is a security power.
        type GuardianOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Hard ceiling on the number of applications.
        #[pallet::constant]
        type MaxApplications: Get<u32>;
        /// Hard ceiling on addresses bound to a single application.
        #[pallet::constant]
        type MaxAddressesPerApp: Get<u32>;
        /// Hard ceiling on versions retained for a single application.
        #[pallet::constant]
        type MaxVersionsPerApp: Get<u32>;
        /// Weights for the dispatchables.
        type WeightInfo: WeightInfo;
    }

    // ── Events ─────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new application was registered at EXPERIMENTAL.
        ApplicationRegistered {
            app_id: ApplicationId,
            owner: T::AccountId,
            vm: GuardianVm,
            bytecode: H256,
        },
        /// A new version was added. The application returns to re-review; new
        /// bytecode can never inherit the previous version's certification.
        ApplicationVersionAdded {
            app_id: ApplicationId,
            version: u32,
            bytecode: H256,
        },
        /// An application was submitted for review.
        SubmittedForReview { app_id: ApplicationId },
        /// An artifact was certified. `bytecode` is the exact hash certified.
        ApplicationCertified {
            app_id: ApplicationId,
            version: u32,
            bytecode: H256,
            next_review: Option<BlockNumberFor<T>>,
        },
        /// An application was restricted (history kept).
        ApplicationRestricted {
            app_id: ApplicationId,
            reason: RestrictionReason,
        },
        /// An application was permanently revoked (history kept).
        ApplicationRevoked {
            app_id: ApplicationId,
            reason: RestrictionReason,
        },
        /// A VM address was bound to an application.
        AddressBound {
            app_id: ApplicationId,
            vm: GuardianVm,
            address: [u8; 32],
        },
        /// A VM address was unbound from an application.
        AddressUnbound {
            app_id: ApplicationId,
            vm: GuardianVm,
            address: [u8; 32],
        },
        /// A Security Manifest was registered; `manifest_hash` is its canonical
        /// hash (spec §23 M3).
        ManifestRegistered {
            app_id: ApplicationId,
            manifest_hash: H256,
        },
    }

    // ── Errors ─────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// No application with this id.
        UnknownApplication,
        /// No version with this index.
        UnknownVersion,
        /// The caller does not own the application.
        NotApplicationOwner,
        /// The application has been permanently revoked.
        ApplicationRevoked,
        /// The application is restricted and cannot be mutated this way.
        ApplicationRestricted,
        /// The application name exceeds `MaxNameLen`.
        NameTooLong,
        /// Would exceed `MaxApplications`.
        TooManyApplications,
        /// Would exceed `MaxVersionsPerApp`.
        TooManyVersions,
        /// Would exceed `MaxAddressesPerApp`.
        TooManyAddresses,
        /// This version index already exists.
        VersionAlreadyExists,
        /// The supplied artifact hashes do not match the stored version — a
        /// certification can never be transferred to different bytecode.
        ArtifactHashMismatch,
        /// Certification may only bind to the current version.
        VersionNotCurrent,
        /// The status transition is not permitted.
        InvalidStatusTransition,
        /// This VM address is already bound to an application.
        AddressAlreadyBound,
        /// This VM address is not bound to this application.
        AddressNotBound,
    }

    // ── Calls ──────────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new application version 0 at EXPERIMENTAL.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_application())]
        pub fn register_application(
            origin: OriginFor<T>,
            name: Vec<u8>,
            vm: GuardianVm,
            category: u16,
            hashes: ArtifactHashes,
            standards: StandardRefs,
        ) -> DispatchResult {
            let owner = T::OwnerOrigin::ensure_origin(origin)?;
            let next = NextApplicationId::<T>::get();
            ensure!(
                next < T::MaxApplications::get() as u64,
                Error::<T>::TooManyApplications
            );
            let name: BoundedName = name.try_into().map_err(|_| Error::<T>::NameTooLong)?;
            let now = frame_system::Pallet::<T>::block_number();

            Applications::<T>::insert(
                next,
                ApplicationRecord {
                    id: next,
                    owner: owner.clone(),
                    name,
                    vm,
                    category,
                    current_version: 0,
                    tier: CertificationTier::Experimental,
                    restriction: None,
                    revoked: false,
                    registered_at: now,
                    updated_at: now,
                    upgrade_count: 0,
                },
            );
            Versions::<T>::insert(
                next,
                0u32,
                VersionRecord {
                    version: 0,
                    hashes,
                    standards,
                    tier: CertificationTier::Experimental,
                    submitted_at: now,
                    certified_at: None,
                    next_review: None,
                    restriction: None,
                },
            );
            NextApplicationId::<T>::put(next.saturating_add(1));

            Self::deposit_event(Event::ApplicationRegistered {
                app_id: next,
                owner,
                vm,
                bytecode: hashes.bytecode,
            });
            Ok(())
        }

        /// Add a new version. The application returns to UNDER_REVIEW: a new
        /// artifact can never inherit the previous version's certification.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::add_version())]
        pub fn add_version(
            origin: OriginFor<T>,
            app_id: ApplicationId,
            hashes: ArtifactHashes,
            standards: StandardRefs,
        ) -> DispatchResult {
            let who = T::OwnerOrigin::ensure_origin(origin)?;
            let mut app = Applications::<T>::get(app_id).ok_or(Error::<T>::UnknownApplication)?;
            ensure!(app.owner == who, Error::<T>::NotApplicationOwner);
            ensure!(!app.revoked, Error::<T>::ApplicationRevoked);
            ensure!(
                app.tier != CertificationTier::Restricted,
                Error::<T>::ApplicationRestricted
            );

            let next = app.current_version.saturating_add(1);
            ensure!(
                next < T::MaxVersionsPerApp::get(),
                Error::<T>::TooManyVersions
            );
            ensure!(
                !Versions::<T>::contains_key(app_id, next),
                Error::<T>::VersionAlreadyExists
            );

            let now = frame_system::Pallet::<T>::block_number();
            Versions::<T>::insert(
                app_id,
                next,
                VersionRecord {
                    version: next,
                    hashes,
                    standards,
                    tier: CertificationTier::UnderReview,
                    submitted_at: now,
                    certified_at: None,
                    next_review: None,
                    restriction: None,
                },
            );
            app.current_version = next;
            app.tier = CertificationTier::UnderReview;
            app.upgrade_count = app.upgrade_count.saturating_add(1);
            app.updated_at = now;
            Applications::<T>::insert(app_id, app);

            Self::deposit_event(Event::ApplicationVersionAdded {
                app_id,
                version: next,
                bytecode: hashes.bytecode,
            });
            Ok(())
        }

        /// Submit the current version for review (EXPERIMENTAL → UNDER_REVIEW).
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::submit_for_review())]
        pub fn submit_for_review(origin: OriginFor<T>, app_id: ApplicationId) -> DispatchResult {
            let who = T::OwnerOrigin::ensure_origin(origin)?;
            let mut app = Applications::<T>::get(app_id).ok_or(Error::<T>::UnknownApplication)?;
            ensure!(app.owner == who, Error::<T>::NotApplicationOwner);
            ensure!(!app.revoked, Error::<T>::ApplicationRevoked);
            ensure!(
                app.tier == CertificationTier::Experimental,
                Error::<T>::InvalidStatusTransition
            );
            app.tier = CertificationTier::UnderReview;
            app.updated_at = frame_system::Pallet::<T>::block_number();
            Applications::<T>::insert(app_id, app);
            Self::deposit_event(Event::SubmittedForReview { app_id });
            Ok(())
        }

        /// Certify the current version, binding X3_VERIFIED to the exact
        /// supplied artifact hashes. Refuses if the hashes do not match the
        /// stored version (certification cannot transfer to other bytecode).
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::certify_application())]
        pub fn certify_application(
            origin: OriginFor<T>,
            app_id: ApplicationId,
            version: u32,
            hashes: ArtifactHashes,
            next_review: Option<BlockNumberFor<T>>,
        ) -> DispatchResult {
            T::GuardianOrigin::ensure_origin(origin)?;
            let mut app = Applications::<T>::get(app_id).ok_or(Error::<T>::UnknownApplication)?;
            ensure!(!app.revoked, Error::<T>::ApplicationRevoked);
            ensure!(
                version == app.current_version,
                Error::<T>::VersionNotCurrent
            );

            let mut ver = Versions::<T>::get(app_id, version).ok_or(Error::<T>::UnknownVersion)?;
            ensure!(ver.hashes == hashes, Error::<T>::ArtifactHashMismatch);

            let now = frame_system::Pallet::<T>::block_number();
            ver.tier = CertificationTier::X3Verified;
            ver.certified_at = Some(now);
            ver.next_review = next_review;
            ver.restriction = None;
            Versions::<T>::insert(app_id, version, ver);

            app.tier = CertificationTier::X3Verified;
            app.restriction = None;
            app.updated_at = now;
            Applications::<T>::insert(app_id, app);

            Self::deposit_event(Event::ApplicationCertified {
                app_id,
                version,
                bytecode: hashes.bytecode,
                next_review,
            });
            Ok(())
        }

        /// Restrict an application: remove its privileges but keep history.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::restrict_application())]
        pub fn restrict_application(
            origin: OriginFor<T>,
            app_id: ApplicationId,
            reason: RestrictionReason,
        ) -> DispatchResult {
            T::GuardianOrigin::ensure_origin(origin)?;
            let mut app = Applications::<T>::get(app_id).ok_or(Error::<T>::UnknownApplication)?;
            ensure!(!app.revoked, Error::<T>::ApplicationRevoked);

            let now = frame_system::Pallet::<T>::block_number();
            app.tier = CertificationTier::Restricted;
            app.restriction = Some(reason);
            app.updated_at = now;
            let current = app.current_version;
            Applications::<T>::insert(app_id, app);

            if let Some(mut ver) = Versions::<T>::get(app_id, current) {
                ver.tier = CertificationTier::Restricted;
                ver.restriction = Some(reason);
                Versions::<T>::insert(app_id, current, ver);
            }

            Self::deposit_event(Event::ApplicationRestricted { app_id, reason });
            Ok(())
        }

        /// Permanently revoke an application and record its current bytecode
        /// (plus source and manifest) as revoked artifacts. History is kept.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::revoke_application())]
        pub fn revoke_application(
            origin: OriginFor<T>,
            app_id: ApplicationId,
            reason: RestrictionReason,
        ) -> DispatchResult {
            T::GuardianOrigin::ensure_origin(origin)?;
            let mut app = Applications::<T>::get(app_id).ok_or(Error::<T>::UnknownApplication)?;
            ensure!(!app.revoked, Error::<T>::ApplicationRevoked);

            let now = frame_system::Pallet::<T>::block_number();
            if let Some(mut ver) = Versions::<T>::get(app_id, app.current_version) {
                ver.tier = CertificationTier::Restricted;
                ver.restriction = Some(reason);
                let hs = ver.hashes;
                Versions::<T>::insert(app_id, app.current_version, ver);
                RevokedArtifacts::<T>::insert(hs.bytecode, now);
                RevokedArtifacts::<T>::insert(hs.source, now);
                RevokedArtifacts::<T>::insert(hs.manifest, now);
            }

            app.tier = CertificationTier::Restricted;
            app.restriction = Some(reason);
            app.revoked = true;
            app.updated_at = now;
            Applications::<T>::insert(app_id, app);

            Self::deposit_event(Event::ApplicationRevoked { app_id, reason });
            Ok(())
        }

        /// Bind a normalized VM address to an application. Fails if the address
        /// is already owned by any application on that VM.
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::bind_address())]
        pub fn bind_address(
            origin: OriginFor<T>,
            app_id: ApplicationId,
            vm: GuardianVm,
            address: [u8; 32],
        ) -> DispatchResult {
            let who = T::OwnerOrigin::ensure_origin(origin)?;
            let app = Applications::<T>::get(app_id).ok_or(Error::<T>::UnknownApplication)?;
            ensure!(app.owner == who, Error::<T>::NotApplicationOwner);
            ensure!(!app.revoked, Error::<T>::ApplicationRevoked);
            ensure!(
                !AddressOwners::<T>::contains_key((vm, address)),
                Error::<T>::AddressAlreadyBound
            );

            let mut list = AppAddresses::<T>::get(app_id).unwrap_or_default();
            list.try_push((vm, address))
                .map_err(|_| Error::<T>::TooManyAddresses)?;
            AppAddresses::<T>::insert(app_id, list);
            AddressOwners::<T>::insert((vm, address), app_id);

            Self::deposit_event(Event::AddressBound {
                app_id,
                vm,
                address,
            });
            Ok(())
        }

        /// Unbind a VM address from an application.
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::unbind_address())]
        pub fn unbind_address(
            origin: OriginFor<T>,
            app_id: ApplicationId,
            vm: GuardianVm,
            address: [u8; 32],
        ) -> DispatchResult {
            let who = T::OwnerOrigin::ensure_origin(origin)?;
            let app = Applications::<T>::get(app_id).ok_or(Error::<T>::UnknownApplication)?;
            ensure!(app.owner == who, Error::<T>::NotApplicationOwner);

            let mut list = AppAddresses::<T>::get(app_id).unwrap_or_default();
            let pos = list
                .iter()
                .position(|a| *a == (vm, address))
                .ok_or(Error::<T>::AddressNotBound)?;
            list.remove(pos);
            AppAddresses::<T>::insert(app_id, list);
            AddressOwners::<T>::remove((vm, address));

            Self::deposit_event(Event::AddressUnbound {
                app_id,
                vm,
                address,
            });
            Ok(())
        }

        /// Register (or replace) the Security Manifest for the current version.
        /// The manifest must describe the exact artifact hashes of the current
        /// version; a manifest for different bytecode is refused. The canonical
        /// hash is recorded so a manifest cannot be swapped without trace
        /// (spec §23 M1–M3, §59 SP5).
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::register_manifest())]
        pub fn register_manifest(
            origin: OriginFor<T>,
            app_id: ApplicationId,
            manifest: SecurityManifest,
        ) -> DispatchResult {
            let who = T::OwnerOrigin::ensure_origin(origin)?;
            let app = Applications::<T>::get(app_id).ok_or(Error::<T>::UnknownApplication)?;
            ensure!(app.owner == who, Error::<T>::NotApplicationOwner);
            ensure!(!app.revoked, Error::<T>::ApplicationRevoked);

            let current = Versions::<T>::get(app_id, app.current_version)
                .ok_or(Error::<T>::UnknownVersion)?;
            ensure!(
                manifest.hashes == current.hashes,
                Error::<T>::ArtifactHashMismatch
            );

            let manifest_hash = manifest.canonical_hash();
            ManifestHashes::<T>::insert(app_id, manifest_hash);
            Manifests::<T>::insert(app_id, manifest);

            Self::deposit_event(Event::ManifestRegistered {
                app_id,
                manifest_hash,
            });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// The current version record of an application, if it exists.
        pub fn current_record(app_id: ApplicationId) -> Option<VersionRecord<T>> {
            let app = Applications::<T>::get(app_id)?;
            Versions::<T>::get(app_id, app.current_version)
        }
    }
}

impl<T: Config> ApplicationRegistryInspect<T::AccountId> for Pallet<T> {
    fn tier(app_id: ApplicationId) -> Option<CertificationTier> {
        Applications::<T>::get(app_id).map(|a| a.tier)
    }

    fn owner(app_id: ApplicationId) -> Option<T::AccountId> {
        Applications::<T>::get(app_id).map(|a| a.owner)
    }

    fn current_version(app_id: ApplicationId) -> Option<u32> {
        Applications::<T>::get(app_id).map(|a| a.current_version)
    }

    fn current_hashes(app_id: ApplicationId) -> Option<ArtifactHashes> {
        Self::current_record(app_id).map(|v| v.hashes)
    }

    fn is_restricted(app_id: ApplicationId) -> Option<bool> {
        Applications::<T>::get(app_id)
            .map(|app| app.revoked || app.tier == CertificationTier::Restricted)
    }

    fn is_certified_artifact(app_id: ApplicationId, bytecode: H256) -> bool {
        let Some(app) = Applications::<T>::get(app_id) else {
            return false;
        };
        if app.revoked || !app.tier.grants_privileges() {
            return false;
        }
        let Some(ver) = Versions::<T>::get(app_id, app.current_version) else {
            return false;
        };
        ver.tier.grants_privileges()
            && ver.hashes.bytecode == bytecode
            && !RevokedArtifacts::<T>::contains_key(bytecode)
    }

    fn has_guardian_privileges(app_id: ApplicationId) -> bool {
        match Applications::<T>::get(app_id) {
            Some(app) => !app.revoked && app.tier.grants_privileges(),
            None => false,
        }
    }

    fn is_artifact_revoked(bytecode: H256) -> bool {
        RevokedArtifacts::<T>::contains_key(bytecode)
    }

    fn manifest_hash(app_id: ApplicationId) -> Option<H256> {
        ManifestHashes::<T>::get(app_id)
    }
}

#[cfg(test)]
mod tests;
