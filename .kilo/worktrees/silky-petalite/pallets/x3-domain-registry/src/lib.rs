#![deny(unsafe_code)]
//! # X3 Domain Registry Pallet
//!
//! Minimal on-chain registry for `.x3` domain ownership and DNS record sets.
//!
//! This pallet provides the canonical source of truth for `.x3` zone records.
//! Networking/DNS serving remains off-chain; consumers query data via runtime API.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

pub mod runtime_api;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use super::runtime_api::{X3DnsRecordResponse, X3DomainResponse, X3RecordType};
    use frame_support::{pallet_prelude::*, traits::StorageVersion, BoundedVec};
    use frame_system::pallet_prelude::*;
    use sp_std::{prelude::*, vec::Vec};

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::without_storage_info]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Origin allowed to set/override records for any domain (e.g. governance).
        type UpdateOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Maximum domain name length in bytes.
        #[pallet::constant]
        type MaxDomainLen: Get<u32>;

        /// Maximum number of domains stored in the global domain list.
        #[pallet::constant]
        type MaxDomains: Get<u32>;

        /// Maximum records stored per domain.
        #[pallet::constant]
        type MaxRecordsPerDomain: Get<u32>;

        /// Maximum CNAME target length.
        #[pallet::constant]
        type MaxCnameLen: Get<u32>;

        /// Maximum TXT record length.
        #[pallet::constant]
        type MaxTxtLen: Get<u32>;
    }

    pub type DomainBytesOf<T> = BoundedVec<u8, <T as Config>::MaxDomainLen>;

    #[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub enum X3RecordData<T: Config> {
        A([u8; 4]),
        Aaaa([u8; 16]),
        Cname(BoundedVec<u8, T::MaxCnameLen>),
        Txt(BoundedVec<u8, T::MaxTxtLen>),
    }

    impl<T: Config> Clone for X3RecordData<T> {
        fn clone(&self) -> Self {
            match self {
                Self::A(v) => Self::A(*v),
                Self::Aaaa(v) => Self::Aaaa(*v),
                Self::Cname(v) => Self::Cname(v.clone()),
                Self::Txt(v) => Self::Txt(v.clone()),
            }
        }
    }

    impl<T: Config> PartialEq for X3RecordData<T> {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (Self::A(a), Self::A(b)) => a == b,
                (Self::Aaaa(a), Self::Aaaa(b)) => a == b,
                (Self::Cname(a), Self::Cname(b)) => a == b,
                (Self::Txt(a), Self::Txt(b)) => a == b,
                _ => false,
            }
        }
    }

    impl<T: Config> Eq for X3RecordData<T> {}

    impl<T: Config> sp_std::fmt::Debug for X3RecordData<T> {
        fn fmt(&self, f: &mut sp_std::fmt::Formatter<'_>) -> sp_std::fmt::Result {
            match self {
                Self::A(v) => f.debug_tuple("A").field(v).finish(),
                Self::Aaaa(v) => f.debug_tuple("AAAA").field(v).finish(),
                Self::Cname(v) => f.debug_tuple("CNAME").field(&v.as_slice()).finish(),
                Self::Txt(v) => f.debug_tuple("TXT").field(&v.as_slice()).finish(),
            }
        }
    }

    impl<T: Config> X3RecordData<T> {
        fn record_type(&self) -> X3RecordType {
            match self {
                X3RecordData::A(_) => X3RecordType::A,
                X3RecordData::Aaaa(_) => X3RecordType::Aaaa,
                X3RecordData::Cname(_) => X3RecordType::Cname,
                X3RecordData::Txt(_) => X3RecordType::Txt,
            }
        }

        fn as_bytes(&self) -> Vec<u8> {
            match self {
                X3RecordData::A(v4) => v4.to_vec(),
                X3RecordData::Aaaa(v6) => v6.to_vec(),
                X3RecordData::Cname(target) => target.to_vec(),
                X3RecordData::Txt(txt) => txt.to_vec(),
            }
        }
    }

    #[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct X3DnsRecord<T: Config> {
        pub ttl: u32,
        pub data: X3RecordData<T>,
    }

    impl<T: Config> Clone for X3DnsRecord<T> {
        fn clone(&self) -> Self {
            Self {
                ttl: self.ttl,
                data: self.data.clone(),
            }
        }
    }

    impl<T: Config> PartialEq for X3DnsRecord<T> {
        fn eq(&self, other: &Self) -> bool {
            self.ttl == other.ttl && self.data == other.data
        }
    }

    impl<T: Config> Eq for X3DnsRecord<T> {}

    impl<T: Config> sp_std::fmt::Debug for X3DnsRecord<T> {
        fn fmt(&self, f: &mut sp_std::fmt::Formatter<'_>) -> sp_std::fmt::Result {
            f.debug_struct("X3DnsRecord")
                .field("ttl", &self.ttl)
                .field("data", &self.data)
                .finish()
        }
    }

    #[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct DomainInfo<T: Config> {
        pub owner: T::AccountId,
        pub records: BoundedVec<X3DnsRecord<T>, T::MaxRecordsPerDomain>,
    }

    #[pallet::storage]
    #[pallet::getter(fn domains)]
    pub type Domains<T: Config> = StorageMap<_, Blake2_128Concat, DomainBytesOf<T>, DomainInfo<T>>;

    #[pallet::storage]
    #[pallet::getter(fn domain_list)]
    pub type DomainList<T: Config> =
        StorageValue<_, BoundedVec<DomainBytesOf<T>, T::MaxDomains>, ValueQuery>;

    #[pallet::error]
    pub enum Error<T> {
        /// Domain name is not under `.x3`.
        NotX3Domain,
        /// Domain already exists.
        DomainAlreadyRegistered,
        /// Domain does not exist.
        DomainNotRegistered,
        /// Caller is not the domain owner.
        NotDomainOwner,
        /// Domain name failed validation.
        InvalidDomain,
        /// Too many records for a domain.
        TooManyRecords,
        /// Record payload invalid.
        InvalidRecord,
        /// Domain list is full.
        DomainListFull,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        DomainRegistered {
            domain: Vec<u8>,
            owner: T::AccountId,
        },
        RecordSet {
            domain: Vec<u8>,
        },
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(frame_support::weights::Weight::from_parts(20_000, 0))]
        pub fn register_domain(origin: OriginFor<T>, domain: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let domain_bounded = Self::validate_x3_domain(domain)?;

            ensure!(
                !Domains::<T>::contains_key(&domain_bounded),
                Error::<T>::DomainAlreadyRegistered
            );

            Domains::<T>::insert(
                &domain_bounded,
                DomainInfo::<T> {
                    owner: who.clone(),
                    records: BoundedVec::default(),
                },
            );

            DomainList::<T>::try_mutate(|list| -> Result<(), Error<T>> {
                if list.iter().any(|d| d == &domain_bounded) {
                    return Ok(());
                }
                list.try_push(domain_bounded.clone())
                    .map_err(|_| Error::<T>::DomainListFull)?;
                Ok(())
            })?;

            Self::deposit_event(Event::DomainRegistered {
                domain: domain_bounded.to_vec(),
                owner: who,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(frame_support::weights::Weight::from_parts(30_000, 0))]
        pub fn set_records(
            origin: OriginFor<T>,
            domain: Vec<u8>,
            records: Vec<X3DnsRecord<T>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let domain_bounded = Self::validate_x3_domain(domain)?;

            Domains::<T>::try_mutate(&domain_bounded, |maybe_info| -> Result<(), Error<T>> {
                let info = maybe_info.as_mut().ok_or(Error::<T>::DomainNotRegistered)?;
                ensure!(info.owner == who, Error::<T>::NotDomainOwner);

                let new_records: BoundedVec<_, T::MaxRecordsPerDomain> =
                    records.try_into().map_err(|_| Error::<T>::TooManyRecords)?;

                // Lightweight validation: ensure record payload shape is correct.
                for rec in new_records.iter() {
                    match &rec.data {
                        X3RecordData::A(_) => {}
                        X3RecordData::Aaaa(_) => {}
                        X3RecordData::Cname(target) => {
                            ensure!(!target.is_empty(), Error::<T>::InvalidRecord);
                        }
                        X3RecordData::Txt(txt) => {
                            ensure!(!txt.is_empty(), Error::<T>::InvalidRecord);
                        }
                    }
                }

                info.records = new_records;
                Ok(())
            })?;

            Self::deposit_event(Event::RecordSet {
                domain: domain_bounded.to_vec(),
            });

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(frame_support::weights::Weight::from_parts(30_000, 0))]
        pub fn set_records_as_governance(
            origin: OriginFor<T>,
            domain: Vec<u8>,
            owner: T::AccountId,
            records: Vec<X3DnsRecord<T>>,
        ) -> DispatchResult {
            T::UpdateOrigin::ensure_origin(origin)?;
            let domain_bounded = Self::validate_x3_domain(domain)?;

            let new_records: BoundedVec<_, T::MaxRecordsPerDomain> =
                records.try_into().map_err(|_| Error::<T>::TooManyRecords)?;

            Domains::<T>::insert(
                &domain_bounded,
                DomainInfo::<T> {
                    owner,
                    records: new_records,
                },
            );

            DomainList::<T>::try_mutate(|list| -> Result<(), Error<T>> {
                if list.iter().any(|d| d == &domain_bounded) {
                    return Ok(());
                }
                list.try_push(domain_bounded.clone())
                    .map_err(|_| Error::<T>::DomainListFull)?;
                Ok(())
            })?;

            Self::deposit_event(Event::RecordSet {
                domain: domain_bounded.to_vec(),
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn validate_x3_domain(domain: Vec<u8>) -> Result<DomainBytesOf<T>, DispatchError> {
            ensure!(!domain.is_empty(), Error::<T>::InvalidDomain);

            // Normalize to lower-case ASCII.
            let mut normalized = Vec::with_capacity(domain.len());
            for b in domain.into_iter() {
                let c = if b.is_ascii_uppercase() { b + 32 } else { b };
                normalized.push(c);
            }

            ensure!(
                Self::is_valid_domain_ascii(&normalized),
                Error::<T>::InvalidDomain
            );
            ensure!(Self::is_x3_domain(&normalized), Error::<T>::NotX3Domain);

            let bounded: DomainBytesOf<T> = normalized
                .try_into()
                .map_err(|_| DispatchError::from(Error::<T>::InvalidDomain))?;

            Ok(bounded)
        }

        fn is_x3_domain(domain: &[u8]) -> bool {
            domain == b"x3" || domain.ends_with(b".x3")
        }

        fn is_valid_domain_ascii(domain: &[u8]) -> bool {
            if domain.len() as u32 > T::MaxDomainLen::get() {
                return false;
            }

            // Very conservative validation: [a-z0-9-] plus dots, no empty labels.
            let mut last_was_dot = true;
            for &b in domain {
                if b == b'.' {
                    if last_was_dot {
                        return false;
                    }
                    last_was_dot = true;
                    continue;
                }

                let ok = b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-';
                if !ok {
                    return false;
                }
                last_was_dot = false;
            }
            !last_was_dot
        }

        pub fn runtime_get_records(domain: Vec<u8>) -> Vec<X3DnsRecordResponse> {
            let Ok(domain_bounded) = Self::validate_x3_domain(domain) else {
                return Vec::new();
            };

            let Some(info) = Domains::<T>::get(&domain_bounded) else {
                return Vec::new();
            };

            info.records
                .into_iter()
                .map(|r| X3DnsRecordResponse {
                    rr_type: r.data.record_type().to_iana_code(),
                    ttl: r.ttl,
                    data: r.data.as_bytes(),
                })
                .collect()
        }

        pub fn runtime_get_domain(domain: Vec<u8>) -> Option<X3DomainResponse<T::AccountId>> {
            let domain_bounded = Self::validate_x3_domain(domain).ok()?;
            let info = Domains::<T>::get(&domain_bounded)?;
            let records = info
                .records
                .iter()
                .map(|r| X3DnsRecordResponse {
                    rr_type: r.data.record_type().to_iana_code(),
                    ttl: r.ttl,
                    data: r.data.as_bytes(),
                })
                .collect();

            Some(X3DomainResponse {
                domain: domain_bounded.to_vec(),
                owner: info.owner,
                records,
            })
        }

        pub fn runtime_list_domains() -> Vec<Vec<u8>> {
            DomainList::<T>::get()
                .into_iter()
                .map(|d| d.to_vec())
                .collect()
        }
    }
}
