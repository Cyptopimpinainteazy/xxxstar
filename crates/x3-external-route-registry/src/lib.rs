use std::collections::HashMap;
use x3_verification_router::{ExternalAssetRef, ExternalChainId, VerificationStrategy};

pub type RouteId = [u8; 32];
pub type AssetId = [u8; 32];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum X3Domain {
    Native,
    Evm,
    Svm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayMode {
    Disabled,
    DryRun,
    TestnetLive,
    GuardedLive,
    FullLive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayRouteConfig {
    pub route_id: RouteId,
    pub external_chain_id: ExternalChainId,
    pub external_asset: ExternalAssetRef,
    pub x3_asset_id: AssetId,
    pub destination_domain: X3Domain,
    pub enabled: bool,
    pub min_amount: u128,
    pub max_amount: u128,
    pub daily_limit: u128,
    pub pending_limit: u32,
    pub finality_requirement: u64,
    pub verification_level: VerificationStrategy,
    pub fee_bps: u16,
    pub mode: GatewayMode,
    pub require_dispute_window: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    AssetNotRegistered,
    RouteNotFound,
    RouteDisabled,
    AmountBelowMinimum,
    AmountAboveMaximum,
    DryRunCannotCredit,
    DisabledMode,
    GuardedLiveRequiresCaps,
    FullLiveRequiresGovernance,
    PrivilegedOriginRequired,
}

#[derive(Debug, Default)]
pub struct ExternalRouteRegistry {
    assets: HashMap<(ExternalChainId, String), AssetId>,
    routes: HashMap<RouteId, GatewayRouteConfig>,
}

impl ExternalRouteRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_external_asset(
        &mut self,
        external_asset: ExternalAssetRef,
        x3_asset_id: AssetId,
    ) {
        self.assets.insert(
            (
                external_asset.chain_id,
                external_asset.token_address_or_mint.clone(),
            ),
            x3_asset_id,
        );
    }

    pub fn enable_gateway_route(
        &mut self,
        mut route_config: GatewayRouteConfig,
    ) -> Result<(), RegistryError> {
        if !self.assets.contains_key(&(
            route_config.external_chain_id,
            route_config.external_asset.token_address_or_mint.clone(),
        )) {
            return Err(RegistryError::AssetNotRegistered);
        }
        Self::validate_mode(&route_config, false)?;
        route_config.enabled = true;
        self.routes.insert(route_config.route_id, route_config);
        Ok(())
    }

    pub fn disable_gateway_route(&mut self, route_id: RouteId) -> Result<(), RegistryError> {
        let route = self
            .routes
            .get_mut(&route_id)
            .ok_or(RegistryError::RouteNotFound)?;
        route.enabled = false;
        Ok(())
    }

    pub fn set_gateway_mode(
        &mut self,
        route_id: RouteId,
        mode: GatewayMode,
        privileged_origin: bool,
    ) -> Result<(), RegistryError> {
        if !privileged_origin {
            return Err(RegistryError::PrivilegedOriginRequired);
        }
        let route = self
            .routes
            .get_mut(&route_id)
            .ok_or(RegistryError::RouteNotFound)?;
        route.mode = mode;
        Self::validate_mode(route, true)
    }

    pub fn get_gateway_route(&self, route_id: RouteId) -> Option<&GatewayRouteConfig> {
        self.routes.get(&route_id)
    }

    pub fn find_route_for_asset(
        &self,
        external_asset: &ExternalAssetRef,
    ) -> Option<&GatewayRouteConfig> {
        self.routes.values().find(|route| {
            route.external_chain_id == external_asset.chain_id
                && route.external_asset.token_address_or_mint
                    == external_asset.token_address_or_mint
        })
    }

    pub fn enforce_route(
        &self,
        route_id: RouteId,
        amount: u128,
    ) -> Result<&GatewayRouteConfig, RegistryError> {
        let route = self
            .routes
            .get(&route_id)
            .ok_or(RegistryError::RouteNotFound)?;
        if !route.enabled {
            return Err(RegistryError::RouteDisabled);
        }
        if route.mode == GatewayMode::Disabled {
            return Err(RegistryError::DisabledMode);
        }
        if route.mode == GatewayMode::DryRun {
            return Err(RegistryError::DryRunCannotCredit);
        }
        if amount < route.min_amount {
            return Err(RegistryError::AmountBelowMinimum);
        }
        if amount > route.max_amount {
            return Err(RegistryError::AmountAboveMaximum);
        }
        Ok(route)
    }

    fn validate_mode(
        route: &GatewayRouteConfig,
        governance_approved: bool,
    ) -> Result<(), RegistryError> {
        match route.mode {
            GatewayMode::GuardedLive if route.max_amount == 0 || route.daily_limit == 0 => {
                Err(RegistryError::GuardedLiveRequiresCaps)
            }
            GatewayMode::FullLive if !governance_approved => {
                Err(RegistryError::FullLiveRequiresGovernance)
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset() -> ExternalAssetRef {
        ExternalAssetRef {
            chain_id: ExternalChainId::BaseSepolia,
            token_address_or_mint: "0xmock".to_string(),
            decimals: 18,
            symbol: "MOCK".to_string(),
        }
    }

    fn route(mode: GatewayMode) -> GatewayRouteConfig {
        GatewayRouteConfig {
            route_id: [1; 32],
            external_chain_id: ExternalChainId::BaseSepolia,
            external_asset: asset(),
            x3_asset_id: [9; 32],
            destination_domain: X3Domain::Native,
            enabled: false,
            min_amount: 1,
            max_amount: 1_000,
            daily_limit: 10_000,
            pending_limit: 10,
            finality_requirement: 32,
            verification_level: VerificationStrategy::ValidatorQuorum,
            fee_bps: 10,
            mode,
            require_dispute_window: false,
        }
    }

    #[test]
    fn route_requires_registered_asset() {
        let mut registry = ExternalRouteRegistry::new();
        assert_eq!(
            registry.enable_gateway_route(route(GatewayMode::TestnetLive)),
            Err(RegistryError::AssetNotRegistered)
        );
    }

    #[test]
    fn dry_run_observes_but_cannot_credit() {
        let mut registry = ExternalRouteRegistry::new();
        registry.register_external_asset(asset(), [9; 32]);
        registry
            .enable_gateway_route(route(GatewayMode::DryRun))
            .unwrap();

        assert_eq!(
            registry.enforce_route([1; 32], 10).unwrap_err(),
            RegistryError::DryRunCannotCredit
        );
    }
}
