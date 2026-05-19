/// Address Book — Contact management with labels and auto-complete
/// Store frequently-used addresses with labels, search by name/tag
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, PartialEq, Eq)]
pub struct ContactInfo {
    pub id: [u8; 32],
    pub owner: [u8; 32],
    pub name: Vec<u8>, // max 50 bytes
    pub address: [u8; 32],
    pub labels: Vec<Vec<u8>>, // e.g., ["exchange", "payment"]
    pub notes: Vec<u8>,       // max 100 bytes
    pub is_verified: bool,
    pub is_favorite: bool,
    pub added_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, PartialEq, Eq)]
pub struct AddressBook {
    pub id: [u8; 32],
    pub owner: [u8; 32],
    pub contacts: Vec<[u8; 32]>, // contact IDs
    pub total_contacts: u32,
    pub created_block: u64,
}

pub struct AddressBookManager;

impl AddressBookManager {
    /// Create address book
    pub fn create_address_book(
        owner: [u8; 32],
        current_block: u64,
    ) -> Result<AddressBook, &'static str> {
        let mut id = [0u8; 32];
        id[0..16].copy_from_slice(&owner[0..16]);

        Ok(AddressBook {
            id,
            owner,
            contacts: vec![],
            total_contacts: 0,
            created_block: current_block,
        })
    }

    /// Add contact to address book
    pub fn add_contact(
        book: &mut AddressBook,
        name: Vec<u8>,
        address: [u8; 32],
        labels: Vec<Vec<u8>>,
        owner: [u8; 32],
        current_block: u64,
    ) -> Result<ContactInfo, &'static str> {
        if owner != book.owner {
            return Err("Only owner can add contacts");
        }
        if name.is_empty() || name.len() > 50 {
            return Err("Invalid contact name length");
        }
        if book.contacts.len() >= 1000 {
            return Err("Address book full");
        }

        let mut contact_id = [0u8; 32];
        contact_id[0..8].copy_from_slice(&owner[0..8]);
        contact_id[8..16].copy_from_slice(&address[0..8]);

        let contact = ContactInfo {
            id: contact_id,
            owner,
            name,
            address,
            labels,
            notes: vec![],
            is_verified: false,
            is_favorite: false,
            added_block: current_block,
        };

        book.contacts.push(contact_id);
        book.total_contacts += 1;

        Ok(contact)
    }

    /// Update contact details
    pub fn update_contact(
        contact: &mut ContactInfo,
        new_name: Vec<u8>,
        new_labels: Vec<Vec<u8>>,
        owner: [u8; 32],
    ) -> Result<(), &'static str> {
        if owner != contact.owner {
            return Err("Only owner can update contact");
        }
        if new_name.is_empty() || new_name.len() > 50 {
            return Err("Invalid name length");
        }

        contact.name = new_name;
        contact.labels = new_labels;
        Ok(())
    }

    /// Mark contact as verified
    pub fn verify_contact(contact: &mut ContactInfo, owner: [u8; 32]) -> Result<(), &'static str> {
        if owner != contact.owner {
            return Err("Only owner can verify");
        }
        contact.is_verified = true;
        Ok(())
    }

    /// Toggle favorite status
    pub fn toggle_favorite(contact: &mut ContactInfo, owner: [u8; 32]) -> Result<(), &'static str> {
        if owner != contact.owner {
            return Err("Only owner can modify");
        }
        contact.is_favorite = !contact.is_favorite;
        Ok(())
    }

    /// Add notes to contact
    pub fn add_notes(
        contact: &mut ContactInfo,
        notes: Vec<u8>,
        owner: [u8; 32],
    ) -> Result<(), &'static str> {
        if owner != contact.owner {
            return Err("Only owner can add notes");
        }
        if notes.len() > 100 {
            return Err("Notes too long");
        }

        contact.notes = notes;
        Ok(())
    }

    /// Search contacts by partial name
    pub fn search_by_name(
        book: &AddressBook,
        search_term: &[u8],
        contacts: &[ContactInfo],
    ) -> Vec<ContactInfo> {
        contacts
            .iter()
            .filter(|c| {
                book.contacts.contains(&c.id)
                    && c.name.windows(search_term.len()).any(|w| w == search_term)
            })
            .cloned()
            .collect()
    }

    /// Get contact by address
    pub fn get_contact_by_address(
        book: &AddressBook,
        address: [u8; 32],
        contacts: &[ContactInfo],
    ) -> Option<ContactInfo> {
        contacts
            .iter()
            .find(|c| book.contacts.contains(&c.id) && c.address == address)
            .cloned()
    }

    /// Get all favorite contacts
    pub fn get_favorites(book: &AddressBook, contacts: &[ContactInfo]) -> Vec<ContactInfo> {
        contacts
            .iter()
            .filter(|c| book.contacts.contains(&c.id) && c.is_favorite)
            .cloned()
            .collect()
    }

    /// Remove contact
    pub fn remove_contact(
        book: &mut AddressBook,
        contact_id: [u8; 32],
        owner: [u8; 32],
    ) -> Result<(), &'static str> {
        if owner != book.owner {
            return Err("Only owner can remove");
        }
        if !book.contacts.contains(&contact_id) {
            return Err("Contact not found");
        }

        book.contacts.retain(|c| c != &contact_id);
        book.total_contacts = book.total_contacts.saturating_sub(1);
        Ok(())
    }

    /// Check if address already in book
    pub fn has_address(book: &AddressBook, address: [u8; 32], contacts: &[ContactInfo]) -> bool {
        contacts
            .iter()
            .any(|c| book.contacts.contains(&c.id) && c.address == address)
    }

    /// Get book size
    pub fn get_contact_count(book: &AddressBook) -> u32 {
        book.total_contacts
    }

    /// Get contacts with label
    pub fn get_contacts_with_label(
        book: &AddressBook,
        label: &[u8],
        contacts: &[ContactInfo],
    ) -> Vec<ContactInfo> {
        contacts
            .iter()
            .filter(|c| book.contacts.contains(&c.id) && c.labels.iter().any(|l| l == label))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_address_book() {
        let result = AddressBookManager::create_address_book([1u8; 32], 100);
        assert!(result.is_ok());
        let book = result.unwrap();
        assert_eq!(book.owner, [1u8; 32]);
        assert_eq!(book.total_contacts, 0);
    }

    #[test]
    fn test_add_contact() {
        let mut book = AddressBookManager::create_address_book([1u8; 32], 100).unwrap();

        let result = AddressBookManager::add_contact(
            &mut book,
            vec![65, 108, 105, 99, 101], // "Alice"
            [2u8; 32],
            vec![],
            [1u8; 32],
            100,
        );
        assert!(result.is_ok());
        assert_eq!(book.total_contacts, 1);
    }

    #[test]
    fn test_add_contact_not_owner() {
        let mut book = AddressBookManager::create_address_book([1u8; 32], 100).unwrap();

        let result = AddressBookManager::add_contact(
            &mut book,
            vec![65],
            [2u8; 32],
            vec![],
            [99u8; 32],
            100,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_add_contact_invalid_name() {
        let mut book = AddressBookManager::create_address_book([1u8; 32], 100).unwrap();

        let result =
            AddressBookManager::add_contact(&mut book, vec![], [2u8; 32], vec![], [1u8; 32], 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_contact() {
        let mut book = AddressBookManager::create_address_book([1u8; 32], 100).unwrap();

        let mut contact =
            AddressBookManager::add_contact(&mut book, vec![65], [2u8; 32], vec![], [1u8; 32], 100)
                .unwrap();

        let result = AddressBookManager::update_contact(
            &mut contact,
            vec![66, 67],                   // "BC"
            vec![vec![116, 101, 115, 116]], // "test"
            [1u8; 32],
        );
        assert!(result.is_ok());
        assert_eq!(contact.name, vec![66, 67]);
    }

    #[test]
    fn test_verify_contact() {
        let mut book = AddressBookManager::create_address_book([1u8; 32], 100).unwrap();

        let mut contact =
            AddressBookManager::add_contact(&mut book, vec![65], [2u8; 32], vec![], [1u8; 32], 100)
                .unwrap();

        assert!(!contact.is_verified);
        let result = AddressBookManager::verify_contact(&mut contact, [1u8; 32]);
        assert!(result.is_ok());
        assert!(contact.is_verified);
    }

    #[test]
    fn test_toggle_favorite() {
        let mut book = AddressBookManager::create_address_book([1u8; 32], 100).unwrap();

        let mut contact =
            AddressBookManager::add_contact(&mut book, vec![65], [2u8; 32], vec![], [1u8; 32], 100)
                .unwrap();

        assert!(!contact.is_favorite);
        AddressBookManager::toggle_favorite(&mut contact, [1u8; 32]).unwrap();
        assert!(contact.is_favorite);
    }

    #[test]
    fn test_add_notes() {
        let mut book = AddressBookManager::create_address_book([1u8; 32], 100).unwrap();

        let mut contact =
            AddressBookManager::add_contact(&mut book, vec![65], [2u8; 32], vec![], [1u8; 32], 100)
                .unwrap();

        let result =
            AddressBookManager::add_notes(&mut contact, vec![78, 111, 116, 101], [1u8; 32]);
        assert!(result.is_ok());
        assert_eq!(contact.notes.len(), 4);
    }

    #[test]
    fn test_remove_contact() {
        let mut book = AddressBookManager::create_address_book([1u8; 32], 100).unwrap();

        let contact =
            AddressBookManager::add_contact(&mut book, vec![65], [2u8; 32], vec![], [1u8; 32], 100)
                .unwrap();

        let result = AddressBookManager::remove_contact(&mut book, contact.id, [1u8; 32]);
        assert!(result.is_ok());
        assert_eq!(book.total_contacts, 0);
    }

    #[test]
    fn test_get_contact_by_address() {
        let mut book = AddressBookManager::create_address_book([1u8; 32], 100).unwrap();

        let contact =
            AddressBookManager::add_contact(&mut book, vec![65], [2u8; 32], vec![], [1u8; 32], 100)
                .unwrap();

        let found =
            AddressBookManager::get_contact_by_address(&book, [2u8; 32], &[contact.clone()]);
        assert!(found.is_some());
        assert_eq!(found.unwrap().address, [2u8; 32]);
    }

    #[test]
    fn test_get_favorites() {
        let mut book = AddressBookManager::create_address_book([1u8; 32], 100).unwrap();

        let mut contact =
            AddressBookManager::add_contact(&mut book, vec![65], [2u8; 32], vec![], [1u8; 32], 100)
                .unwrap();

        AddressBookManager::toggle_favorite(&mut contact, [1u8; 32]).unwrap();

        let favorites = AddressBookManager::get_favorites(&book, &[contact]);
        assert_eq!(favorites.len(), 1);
    }

    #[test]
    fn test_has_address() {
        let mut book = AddressBookManager::create_address_book([1u8; 32], 100).unwrap();

        let contact =
            AddressBookManager::add_contact(&mut book, vec![65], [2u8; 32], vec![], [1u8; 32], 100)
                .unwrap();

        assert!(AddressBookManager::has_address(
            &book,
            [2u8; 32],
            &[contact.clone()]
        ));
        assert!(!AddressBookManager::has_address(
            &book,
            [99u8; 32],
            &[contact]
        ));
    }

    #[test]
    fn test_get_contact_count() {
        let mut book = AddressBookManager::create_address_book([1u8; 32], 100).unwrap();

        AddressBookManager::add_contact(&mut book, vec![65], [2u8; 32], vec![], [1u8; 32], 100)
            .unwrap();

        AddressBookManager::add_contact(&mut book, vec![66], [3u8; 32], vec![], [1u8; 32], 100)
            .unwrap();

        assert_eq!(AddressBookManager::get_contact_count(&book), 2);
    }

    #[test]
    fn test_search_by_name() {
        let mut book = AddressBookManager::create_address_book([1u8; 32], 100).unwrap();

        let _contact = AddressBookManager::add_contact(
            &mut book,
            vec![65, 108, 105, 99, 101], // Alice
            [2u8; 32],
            vec![],
            [1u8; 32],
            100,
        )
        .unwrap();

        let contacts = vec![_contact];
        let results = AddressBookManager::search_by_name(&book, &[108, 105], &contacts);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_get_contacts_with_label() {
        let mut book = AddressBookManager::create_address_book([1u8; 32], 100).unwrap();

        let contact = AddressBookManager::add_contact(
            &mut book,
            vec![65],
            [2u8; 32],
            vec![vec![101, 120, 99, 104, 97, 110, 103, 101]], // "exchange"
            [1u8; 32],
            100,
        )
        .unwrap();

        let results = AddressBookManager::get_contacts_with_label(
            &book,
            &[101, 120, 99, 104, 97, 110, 103, 101],
            &[contact],
        );
        assert_eq!(results.len(), 1);
    }
}
