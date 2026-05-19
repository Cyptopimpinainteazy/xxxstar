//! P2P Network Integration Tests

#[cfg(test)]
mod network_tests {
    use gpu_swarm::network::{NetworkConfig, NetworkManager, PeerId};
    use std::time::Duration;

    #[tokio::test]
    async fn test_network_manager_creation() {
        let config = NetworkConfig::default();
        let manager = NetworkManager::new(config);

        assert!(manager.is_ok());
        let mgr = manager.unwrap();
        assert_eq!(mgr.peer_count(), 0);
    }

    #[tokio::test]
    async fn test_peer_connection() {
        let config = NetworkConfig::default();
        let mut manager = NetworkManager::new(config).unwrap();
        manager.start().await.unwrap();

        let peer_id = manager.connect("/ip4/127.0.0.1/tcp/9000").await.unwrap();

        assert_eq!(manager.peer_count(), 1);
        assert!(manager.get_peer_info(&peer_id).is_some());
    }

    #[tokio::test]
    async fn test_peer_reputation() {
        let config = NetworkConfig::default();
        let mut manager = NetworkManager::new(config).unwrap();

        let peer_id = manager.connect("/ip4/127.0.0.1/tcp/9001").await.unwrap();

        // Initial reputation
        let info = manager.get_peer_info(&peer_id).unwrap();
        let initial_score = info.reputation_score;

        // Update reputation with successes
        for _ in 0..5 {
            manager.update_reputation(&peer_id, true);
        }

        let updated = manager.get_peer_info(&peer_id).unwrap();
        assert!(updated.reputation_score >= initial_score);
    }

    #[tokio::test]
    async fn test_peer_blacklisting() {
        let config = NetworkConfig::default();
        let mut manager = NetworkManager::new(config).unwrap();

        let peer_id = manager.connect("/ip4/127.0.0.1/tcp/9002").await.unwrap();

        assert!(!manager.get_peer_info(&peer_id).unwrap().is_blacklisted);

        manager.blacklist_peer(&peer_id);
        assert!(manager.get_peer_info(&peer_id).unwrap().is_blacklisted);

        // Unblacklist
        manager.unblacklist_peer(&peer_id);
        assert!(!manager.get_peer_info(&peer_id).unwrap().is_blacklisted);
    }

    #[tokio::test]
    async fn test_healthy_peers_filtering() {
        let config = NetworkConfig::default();
        let mut manager = NetworkManager::new(config).unwrap();

        // Add a good peer
        let good_peer = manager.connect("/ip4/127.0.0.1/tcp/9003").await.unwrap();

        for _ in 0..10 {
            manager.update_reputation(&good_peer, true);
        }

        // Add a bad peer
        let bad_peer = manager.connect("/ip4/127.0.0.1/tcp/9004").await.unwrap();

        for _ in 0..10 {
            manager.update_reputation(&bad_peer, false);
        }

        let healthy = manager.healthy_peers();
        assert!(healthy.len() > 0);
    }

    #[tokio::test]
    async fn test_message_broadcast() {
        let config = NetworkConfig::default();
        let manager = NetworkManager::new(config).unwrap();

        manager
            .broadcast(gpu_swarm::protocol::SwarmMessage::Ping)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_peer_discovery() {
        use gpu_swarm::network::PeerDiscovery;

        let local_id = PeerId::random();
        let mut discovery = PeerDiscovery::new(vec![], local_id);

        let peer_id = PeerId::random();
        discovery.add_peer(peer_id, vec!["/ip4/127.0.0.1/tcp/9010".to_string()]);

        assert!(discovery.find_peer(&peer_id).is_some());
    }

    #[test]
    fn test_connection_manager() {
        use gpu_swarm::network::{ConnectionDirection, ConnectionManager};

        let manager = ConnectionManager::new(10);
        let peer_id = PeerId::random();

        assert!(manager.can_accept());
        assert!(manager.add(
            peer_id,
            "/ip4/127.0.0.1/tcp/9005".to_string(),
            ConnectionDirection::Outbound
        ));
        assert_eq!(manager.count(), 1);

        manager.update_bytes(&peer_id, 100, 50);

        let conn = manager.get(&peer_id).unwrap();
        assert_eq!(conn.bytes_sent, 100);
        assert_eq!(conn.bytes_received, 50);
    }

    #[test]
    fn test_multi_connection_limits() {
        use gpu_swarm::network::{ConnectionDirection, ConnectionManager};

        let manager = ConnectionManager::new(3);

        for i in 0..3 {
            let peer_id = PeerId::random();
            assert!(manager.add(
                peer_id,
                format!("/ip4/127.0.0.1/tcp/{}", 9010 + i),
                ConnectionDirection::Inbound
            ));
        }

        let peer_id = PeerId::random();
        assert!(!manager.add(
            peer_id,
            "/ip4/127.0.0.1/tcp/9020".to_string(),
            ConnectionDirection::Inbound
        ));
    }

    #[test]
    fn test_network_config() {
        let mut config = NetworkConfig::default();

        assert!(!config.listen_addresses.is_empty());
        assert_eq!(config.max_incoming, 100);
        assert_eq!(config.max_outgoing, 50);

        config.max_peers = 1000;
        assert_eq!(config.max_peers, 1000);
    }
}
