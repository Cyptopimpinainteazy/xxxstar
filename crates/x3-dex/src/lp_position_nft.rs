/// LP Position NFT Engine — Mint liquidity provider positions as tradeable NFTs
/// Enables LP position ownership transfer, collateralization, and secondary market trading
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::prelude::*;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct LPPositionNFT {
    pub nft_id: [u8; 32],
    pub position_id: [u8; 32],
    pub owner: [u8; 32],
    pub token_a: u128,
    pub token_b: u128,
    pub liquidity_amount: u64,
    pub lower_tick: i32,
    pub upper_tick: i32,
    pub fee_tier: u32,
    pub minted_block: u64,
    pub minting_timestamp: u64,
    pub is_active: bool,
    pub accumulated_fees: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct NFTMetadata {
    pub nft_id: [u8; 32],
    pub name: Vec<u8>,
    pub description: Vec<u8>,
    pub image_uri: Vec<u8>,
    pub attributes: Vec<(Vec<u8>, Vec<u8>)>, // key-value pairs
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct NFTTransfer {
    pub nft_id: [u8; 32],
    pub from: [u8; 32],
    pub to: [u8; 32],
    pub price: u64,
    pub transfer_block: u64,
    pub transfer_hash: [u8; 32],
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct NFTListing {
    pub listing_id: [u8; 32],
    pub nft_id: [u8; 32],
    pub seller: [u8; 32],
    pub price: u64,
    pub active: bool,
    pub created_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct NFTCollateral {
    pub collateral_id: [u8; 32],
    pub nft_id: [u8; 32],
    pub borrower: [u8; 32],
    pub loan_amount: u64,
    pub collateral_value: u64,
    pub ltv_ratio: u32, // loan-to-value: 3333 = 33.33%
    pub liquidation_price: u64,
    pub is_liquidated: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct NFTRoyalty {
    pub nft_id: [u8; 32],
    pub creator: [u8; 32],
    pub royalty_bps: u32,
    pub total_royalties_paid: u64,
}

pub struct LPPositionNFTEngine;

impl LPPositionNFTEngine {
    const MIN_LIQUIDITY_FOR_NFT: u64 = 1_000; // Minimum liquidity to mint NFT
    const MAX_ROYALTY_BPS: u32 = 2_000; // 20% max royalty
    const MIN_LTV_RATIO: u32 = 2_500; // 25% minimum loan-to-value
    const MAX_LTV_RATIO: u32 = 7_500; // 75% maximum loan-to-value

    /// Mint LP position as NFT
    #[allow(clippy::too_many_arguments)]
    pub fn mint_lp_position_nft(
        position_id: [u8; 32],
        owner: [u8; 32],
        token_a: u128,
        token_b: u128,
        liquidity: u64,
        lower_tick: i32,
        upper_tick: i32,
        fee_tier: u32,
        current_block: u64,
        timestamp: u64,
    ) -> Result<LPPositionNFT, &'static str> {
        if liquidity < Self::MIN_LIQUIDITY_FOR_NFT {
            return Err("Liquidity below minimum NFT threshold");
        }

        if lower_tick >= upper_tick {
            return Err("Invalid tick range");
        }

        let nft = LPPositionNFT {
            nft_id: Self::derive_nft_id(position_id, owner),
            position_id,
            owner,
            token_a,
            token_b,
            liquidity_amount: liquidity,
            lower_tick,
            upper_tick,
            fee_tier,
            minted_block: current_block,
            minting_timestamp: timestamp,
            is_active: true,
            accumulated_fees: 0,
        };

        Ok(nft)
    }

    /// Burn NFT (remove from circulation, position remains or burns with it)
    pub fn burn_nft(nft: &mut LPPositionNFT, caller: [u8; 32]) -> Result<bool, &'static str> {
        if nft.owner != caller {
            return Err("Only NFT owner can burn");
        }

        nft.is_active = false;
        Ok(true)
    }

    /// Transfer NFT to new owner
    pub fn transfer_nft(
        nft: &mut LPPositionNFT,
        from: [u8; 32],
        to: [u8; 32],
        transfer_block: u64,
    ) -> Result<NFTTransfer, &'static str> {
        if nft.owner != from {
            return Err("Caller is not NFT owner");
        }

        if !nft.is_active {
            return Err("Cannot transfer inactive NFT");
        }

        nft.owner = to;

        let transfer = NFTTransfer {
            nft_id: nft.nft_id,
            from,
            to,
            price: nft.liquidity_amount,
            transfer_block,
            transfer_hash: Self::derive_transfer_hash(nft.nft_id, from, to),
        };

        Ok(transfer)
    }

    /// Create NFT listing at price
    pub fn list_nft(
        nft: &LPPositionNFT,
        price: u64,
        current_block: u64,
    ) -> Result<NFTListing, &'static str> {
        if !nft.is_active {
            return Err("Cannot list inactive NFT");
        }

        if price == 0 {
            return Err("Listing price must be > 0");
        }

        let listing = NFTListing {
            listing_id: Self::derive_listing_id(nft.nft_id, price, current_block),
            nft_id: nft.nft_id,
            seller: nft.owner,
            price,
            active: true,
            created_block: current_block,
        };

        Ok(listing)
    }

    /// Cancel NFT listing
    pub fn cancel_listing(
        listing: &mut NFTListing,
        caller: [u8; 32],
    ) -> Result<bool, &'static str> {
        if listing.seller != caller {
            return Err("Only seller can cancel listing");
        }

        listing.active = false;
        Ok(true)
    }

    /// Buy listed NFT
    pub fn buy_nft(
        nft: &mut LPPositionNFT,
        listing: &mut NFTListing,
        buyer: [u8; 32],
        payment: u64,
        creator_royalty_bps: u32,
    ) -> Result<(NFTTransfer, u64, u64), &'static str> {
        if !listing.active {
            return Err("Listing is not active");
        }

        if payment < listing.price {
            return Err("Insufficient payment");
        }

        // Calculate royalty
        let royalty = (listing.price as u128 * creator_royalty_bps as u128 / 10_000) as u64;
        let seller_proceeds = listing.price - royalty;

        nft.owner = buyer;

        let transfer = NFTTransfer {
            nft_id: nft.nft_id,
            from: listing.seller,
            to: buyer,
            price: listing.price,
            transfer_block: 0,
            transfer_hash: Self::derive_transfer_hash(nft.nft_id, listing.seller, buyer),
        };

        listing.active = false;

        Ok((transfer, seller_proceeds, royalty))
    }

    /// Use NFT as collateral for loan
    pub fn collateralize_nft(
        nft: &LPPositionNFT,
        borrower: [u8; 32],
        loan_amount: u64,
        ltv_ratio: u32,
    ) -> Result<NFTCollateral, &'static str> {
        if !nft.is_active {
            return Err("Cannot collateralize inactive NFT");
        }

        if !(Self::MIN_LTV_RATIO..=Self::MAX_LTV_RATIO).contains(&ltv_ratio) {
            return Err("LTV ratio out of bounds");
        }

        // Calculate collateral value: loan / (ltv / 10000)
        let collateral_value = (loan_amount as u128 * 10_000 / ltv_ratio as u128) as u64;

        if collateral_value > nft.liquidity_amount * 2 {
            return Err("Loan amount exceeds NFT value");
        }

        let liquidation_price = (collateral_value as u128 * 7_500 / 10_000) as u64; // 75% of collateral

        let collateral = NFTCollateral {
            collateral_id: Self::derive_collateral_id(nft.nft_id, borrower),
            nft_id: nft.nft_id,
            borrower,
            loan_amount,
            collateral_value,
            ltv_ratio,
            liquidation_price,
            is_liquidated: false,
        };

        Ok(collateral)
    }

    /// Update accumulated fees on NFT
    pub fn update_accumulated_fees(
        nft: &mut LPPositionNFT,
        new_fees: u64,
    ) -> Result<u64, &'static str> {
        nft.accumulated_fees = nft.accumulated_fees.saturating_add(new_fees);
        Ok(nft.accumulated_fees)
    }

    /// Claim accumulated fees from NFT
    pub fn claim_nft_fees(nft: &mut LPPositionNFT, caller: [u8; 32]) -> Result<u64, &'static str> {
        if nft.owner != caller {
            return Err("Only owner can claim fees");
        }

        let fees = nft.accumulated_fees;
        nft.accumulated_fees = 0;

        Ok(fees)
    }

    /// Create NFT metadata for marketplace display
    pub fn create_nft_metadata(
        nft_id: [u8; 32],
        name: Vec<u8>,
        description: Vec<u8>,
        image_uri: Vec<u8>,
    ) -> Result<NFTMetadata, &'static str> {
        if name.is_empty() || description.is_empty() {
            return Err("Name and description required");
        }

        let metadata = NFTMetadata {
            nft_id,
            name,
            description,
            image_uri,
            attributes: Vec::new(),
        };

        Ok(metadata)
    }

    /// Add attributes to NFT metadata
    pub fn add_metadata_attribute(
        metadata: &mut NFTMetadata,
        key: Vec<u8>,
        value: Vec<u8>,
    ) -> Result<(), &'static str> {
        if key.is_empty() {
            return Err("Attribute key cannot be empty");
        }

        metadata.attributes.push((key, value));
        Ok(())
    }

    /// Set creator royalty
    pub fn set_creator_royalty(
        nft_id: [u8; 32],
        creator: [u8; 32],
        royalty_bps: u32,
    ) -> Result<NFTRoyalty, &'static str> {
        if royalty_bps > Self::MAX_ROYALTY_BPS {
            return Err("Royalty exceeds maximum");
        }

        let royalty = NFTRoyalty {
            nft_id,
            creator,
            royalty_bps,
            total_royalties_paid: 0,
        };

        Ok(royalty)
    }

    /// Track royalty payment
    pub fn track_royalty_payment(
        royalty: &mut NFTRoyalty,
        amount: u64,
    ) -> Result<u64, &'static str> {
        royalty.total_royalties_paid = royalty.total_royalties_paid.saturating_add(amount);
        Ok(royalty.total_royalties_paid)
    }

    /// Calculate NFT value based on liquidity and accumulated fees
    pub fn calculate_nft_value(nft: &LPPositionNFT) -> u64 {
        nft.liquidity_amount.saturating_add(nft.accumulated_fees)
    }

    /// Check if NFT is underwater (collateral value < loan + interest)
    pub fn is_underwater(collateral: &NFTCollateral, current_price: u64) -> bool {
        current_price <= collateral.liquidation_price
    }

    /// Derive deterministic NFT ID
    fn derive_nft_id(position_id: [u8; 32], owner: [u8; 32]) -> [u8; 32] {
        let mut nft_id = [0u8; 32];
        for (i, byte) in position_id.iter().enumerate().take(16) {
            nft_id[i] = *byte;
        }
        for (i, byte) in owner.iter().enumerate().take(16) {
            nft_id[i + 16] = *byte;
        }
        nft_id
    }

    /// Derive transfer hash
    fn derive_transfer_hash(nft_id: [u8; 32], from: [u8; 32], to: [u8; 32]) -> [u8; 32] {
        let mut hash = [0u8; 32];
        for (i, byte) in nft_id.iter().enumerate().take(11) {
            hash[i] = *byte;
        }
        for (i, byte) in from.iter().enumerate().take(10) {
            hash[i + 11] = *byte;
        }
        for (i, byte) in to.iter().enumerate().take(11) {
            hash[i + 21] = *byte;
        }
        hash
    }

    /// Derive listing ID
    fn derive_listing_id(nft_id: [u8; 32], price: u64, block: u64) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in nft_id.iter().enumerate().take(16) {
            id[i] = *byte;
        }
        let price_bytes = price.to_le_bytes();
        for (i, byte) in price_bytes.iter().enumerate().take(8) {
            id[i + 16] = *byte;
        }
        let block_bytes = block.to_le_bytes();
        for (i, byte) in block_bytes.iter().take(8).enumerate() {
            id[i + 24] = *byte;
        }
        id
    }

    /// Derive collateral ID
    fn derive_collateral_id(nft_id: [u8; 32], borrower: [u8; 32]) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in nft_id.iter().enumerate() {
            id[i] = *byte ^ borrower[i];
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mint_lp_position_nft() {
        let nft = LPPositionNFTEngine::mint_lp_position_nft(
            [1; 32], [2; 32], 1, 2, 50_000, -100, 100, 3000, 100, 1_000_000,
        )
        .unwrap();

        assert_eq!(nft.liquidity_amount, 50_000);
        assert!(nft.is_active);
    }

    #[test]
    fn test_burn_nft() {
        let mut nft = LPPositionNFTEngine::mint_lp_position_nft(
            [1; 32], [2; 32], 1, 2, 50_000, -100, 100, 3000, 100, 1_000_000,
        )
        .unwrap();

        let owner = nft.owner;
        LPPositionNFTEngine::burn_nft(&mut nft, owner).unwrap();
        assert!(!nft.is_active);
    }

    #[test]
    fn test_transfer_nft() {
        let mut nft = LPPositionNFTEngine::mint_lp_position_nft(
            [1; 32], [2; 32], 1, 2, 50_000, -100, 100, 3000, 100, 1_000_000,
        )
        .unwrap();

        let owner = nft.owner;
        let transfer = LPPositionNFTEngine::transfer_nft(&mut nft, owner, [3; 32], 200).unwrap();

        assert_eq!(nft.owner, [3; 32]);
        assert_eq!(transfer.to, [3; 32]);
    }

    #[test]
    fn test_list_nft() {
        let nft = LPPositionNFTEngine::mint_lp_position_nft(
            [1; 32], [2; 32], 1, 2, 50_000, -100, 100, 3000, 100, 1_000_000,
        )
        .unwrap();

        let listing = LPPositionNFTEngine::list_nft(&nft, 75_000, 200).unwrap();

        assert!(listing.active);
        assert_eq!(listing.price, 75_000);
    }

    #[test]
    fn test_buy_nft() {
        let mut nft = LPPositionNFTEngine::mint_lp_position_nft(
            [1; 32], [2; 32], 1, 2, 50_000, -100, 100, 3000, 100, 1_000_000,
        )
        .unwrap();

        let mut listing = LPPositionNFTEngine::list_nft(&nft, 75_000, 200).unwrap();

        let (_transfer, _seller_proceeds, royalty) = LPPositionNFTEngine::buy_nft(
            &mut nft,
            &mut listing,
            [4; 32],
            75_000,
            500, // 5% royalty
        )
        .unwrap();

        assert_eq!(nft.owner, [4; 32]);
        assert!(royalty > 0);
    }

    #[test]
    fn test_collateralize_nft() {
        let nft = LPPositionNFTEngine::mint_lp_position_nft(
            [1; 32], [2; 32], 1, 2, 50_000, -100, 100, 3000, 100, 1_000_000,
        )
        .unwrap();

        let collateral = LPPositionNFTEngine::collateralize_nft(
            &nft, [2; 32], 25_000, 5_000, // 50% LTV
        )
        .unwrap();

        assert_eq!(collateral.loan_amount, 25_000);
    }

    #[test]
    fn test_update_accumulated_fees() {
        let mut nft = LPPositionNFTEngine::mint_lp_position_nft(
            [1; 32], [2; 32], 1, 2, 50_000, -100, 100, 3000, 100, 1_000_000,
        )
        .unwrap();

        let fees = LPPositionNFTEngine::update_accumulated_fees(&mut nft, 1_000).unwrap();

        assert_eq!(fees, 1_000);
    }

    #[test]
    fn test_claim_nft_fees() {
        let mut nft = LPPositionNFTEngine::mint_lp_position_nft(
            [1; 32], [2; 32], 1, 2, 50_000, -100, 100, 3000, 100, 1_000_000,
        )
        .unwrap();

        LPPositionNFTEngine::update_accumulated_fees(&mut nft, 2_000).unwrap();
        let owner = nft.owner;
        let claimed = LPPositionNFTEngine::claim_nft_fees(&mut nft, owner).unwrap();

        assert_eq!(claimed, 2_000);
        assert_eq!(nft.accumulated_fees, 0);
    }

    #[test]
    fn test_create_nft_metadata() {
        let metadata = LPPositionNFTEngine::create_nft_metadata(
            [1; 32],
            b"LP Position #1".to_vec(),
            b"My liquidity position".to_vec(),
            b"ipfs://...".to_vec(),
        )
        .unwrap();

        assert!(!metadata.name.is_empty());
    }

    #[test]
    fn test_add_metadata_attribute() {
        let mut metadata = LPPositionNFTEngine::create_nft_metadata(
            [1; 32],
            b"LP Position #1".to_vec(),
            b"My liquidity position".to_vec(),
            b"ipfs://...".to_vec(),
        )
        .unwrap();

        LPPositionNFTEngine::add_metadata_attribute(
            &mut metadata,
            b"fee_tier".to_vec(),
            b"3000".to_vec(),
        )
        .unwrap();

        assert_eq!(metadata.attributes.len(), 1);
    }

    #[test]
    fn test_set_creator_royalty() {
        let royalty = LPPositionNFTEngine::set_creator_royalty(
            [1; 32], [2; 32], 500, // 5%
        )
        .unwrap();

        assert_eq!(royalty.royalty_bps, 500);
    }

    #[test]
    fn test_is_underwater() {
        let nft = LPPositionNFTEngine::mint_lp_position_nft(
            [1; 32], [2; 32], 1, 2, 50_000, -100, 100, 3000, 100, 1_000_000,
        )
        .unwrap();

        let collateral =
            LPPositionNFTEngine::collateralize_nft(&nft, [2; 32], 25_000, 5_000).unwrap();

        let underwater = LPPositionNFTEngine::is_underwater(&collateral, 37_000);

        assert!(underwater);
    }

    #[test]
    fn test_calculate_nft_value() {
        let mut nft = LPPositionNFTEngine::mint_lp_position_nft(
            [1; 32], [2; 32], 1, 2, 50_000, -100, 100, 3000, 100, 1_000_000,
        )
        .unwrap();

        nft.accumulated_fees = 2_000;

        let value = LPPositionNFTEngine::calculate_nft_value(&nft);

        assert_eq!(value, 52_000);
    }

    #[test]
    fn test_cancel_listing() {
        let nft = LPPositionNFTEngine::mint_lp_position_nft(
            [1; 32], [2; 32], 1, 2, 50_000, -100, 100, 3000, 100, 1_000_000,
        )
        .unwrap();

        let mut listing = LPPositionNFTEngine::list_nft(&nft, 75_000, 200).unwrap();

        LPPositionNFTEngine::cancel_listing(&mut listing, nft.owner).unwrap();

        assert!(!listing.active);
    }
}
