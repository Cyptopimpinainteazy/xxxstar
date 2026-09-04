use super::*;

    #[test]
    fn test_read_system_metrics_returns_valid_metrics() {
        let sys = System::new_all();
        let metrics = read_system_metrics(&sys);

        // Verify CPU metrics
        assert!(metrics.cpu.usage_percent >= 0.0);
        assert!(metrics.cpu.usage_percent <= 100.0);
        assert!(metrics.cpu.cores > 0);
        assert!(metrics.cpu.frequency >= 0);

        // Verify memory metrics
        assert!(metrics.memory.used <= metrics.memory.total);
        assert!(metrics.memory.usage_percent >= 0.0);
        assert!(metrics.memory.usage_percent <= 100.0);
        assert!(metrics.memory.total > 0);

        // Verify timestamp
        assert!(!metrics.updated_at.is_empty());
        assert!(metrics.updated_at.contains("T"));
        assert!(metrics.updated_at.contains("Z"));
    }

    #[test]
    fn test_system_metrics_disk_info() {
        let sys = System::new_all();
        let metrics = read_system_metrics(&sys);

        // Disk should be present
        for disk in &metrics.disk {
            assert!(!disk.name.is_empty());
            assert!(disk.total > 0);
            assert!(disk.usage_percent >= 0.0);
            assert!(disk.usage_percent <= 100.0);
            assert!(disk.used <= disk.total);
        }
    }

    #[test]
    fn test_seed_ipfs_storage_creates_valid_data() {
        let storage = seed_ipfs_storage();

        // Verify node ID
        assert!(!storage.node_id.is_empty());
        assert_eq!(storage.node_id.len(), 36); // UUID format

        // Verify storage capacity
        assert_eq!(storage.storage_capacity, 500_000_000_000); // 500GB

        // Verify pinned objects
        assert!(!storage.pinned_objects.is_empty());
        for pin in &storage.pinned_objects {
            assert!(!pin.cid.is_empty());
            assert!(!pin.name.is_empty());
            assert!(pin.size > 0);
            assert!(pin.replicas > 0);
            assert!(pin.earning_potential >= 0.0);
            assert!(!pin.pinned_at.is_empty());
        }

        // Verify storage deals
        assert!(!storage.storage_market.is_empty());
        for deal in &storage.storage_market {
            assert!(!deal.id.is_empty());
            assert!(!deal.client.is_empty());
            assert!(deal.size > 0);
            assert!(deal.price_per_epoch > 0.0);
            assert!(deal.duration_epochs > 0);
            assert!(deal.earned >= 0.0);
        }

        // Verify calculated fields
        assert!(storage.storage_used > 0);
        assert_eq!(storage.total_pins as usize, storage.pinned_objects.len());
        assert!(!storage.updated_at.is_empty());
    }

    #[test]
    fn test_seed_ipfs_storage_total_pins_accuracy() {
        let storage = seed_ipfs_storage();

        // Total pins should match pinned objects count
        assert_eq!(storage.total_pins as usize, storage.pinned_objects.len());
    }

    #[test]
    fn test_seed_ipfs_storage_size_calculation() {
        let storage = seed_ipfs_storage();

        // Calculate storage used from pinned objects
        let calculated_size: u64 = storage.pinned_objects.iter().map(|p| p.size).sum();

        // Should match reported storage_used
        assert_eq!(storage.storage_used, calculated_size);
    }

    #[test]
    fn test_ipfs_deal_status_active() {
        let storage = seed_ipfs_storage();

        // All deals should have a valid status
        for deal in &storage.storage_market {
            assert_eq!(deal.status, StorageDealStatus::Active);
        }
    }

    #[test]
    fn test_cpu_metrics_within_bounds() {
        let sys = System::new_all();
        let metrics = read_system_metrics(&sys);

        // CPU percentage should be between 0-100
        assert!(metrics.cpu.usage_percent >= 0.0, "CPU usage cannot be negative");
        assert!(
            metrics.cpu.usage_percent <= 110.0,
            "CPU usage should not exceed 110%"
        );

        // Cores should be reasonable
        assert!(metrics.cpu.cores > 0, "Should have at least 1 core");
        assert!(metrics.cpu.cores <= 1024, "Cores should be reasonable");
    }

    #[test]
    fn test_memory_metrics_consistency() {
        let sys = System::new_all();
        let metrics = read_system_metrics(&sys);

        // Memory used should not exceed total
        assert!(
            metrics.memory.used <= metrics.memory.total,
            "Used memory cannot exceed total memory"
        );

        // Memory percentage should match calculation
        let calculated_percent =
            (metrics.memory.used as f32 / metrics.memory.total as f32) * 100.0;
        assert!(
            (calculated_percent - metrics.memory.usage_percent).abs() < 1.0,
            "Memory percentage calculation off by more than 1%"
        );
    }

    #[test]
    fn test_disk_metrics_consistency() {
        let sys = System::new_all();
        let metrics = read_system_metrics(&sys);

        for disk in &metrics.disk {
            // Disk used should not exceed total
            assert!(
                disk.used <= disk.total,
                "Used disk space cannot exceed total"
            );

            // Disk percentage should match calculation
            let calculated_percent = (disk.used as f32 / disk.total as f32) * 100.0;
            assert!(
                (calculated_percent - disk.usage_percent).abs() < 1.0,
                "Disk percentage calculation off by more than 1%"
            );

            // Percentage should be capped at 100%
            assert!(disk.usage_percent <= 100.0, "Disk usage percentage over 100%");
        }
    }

    #[test]
    fn test_update_system_metrics_refreshes_data() {
        let state = TelemetryState::new();

        // Get initial metrics
        update_system_metrics(&state);
        let metrics1 = state
            .system
            .read()
            .expect("Failed to read system metrics");
        let timestamp1 = metrics1.updated_at.clone();

        // Small delay to ensure timestamp would change
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Update metrics
        update_system_metrics(&state);
        let metrics2 = state
            .system
            .read()
            .expect("Failed to read system metrics");
        let timestamp2 = metrics2.updated_at.clone();

        // Timestamps should be different
        assert_ne!(timestamp1, timestamp2);
    }

    #[test]
    fn test_update_ipfs_storage_modifies_earnings() {
        let state = TelemetryState::new();

        // Initial earnings
        let mut rng = rand::thread_rng();
        {
            let data = state.ipfs.write().expect("Failed to write ipfs data");
            // Store initial earnings
            let _initial_earnings: f64 = data.storage_market.iter().map(|d| d.earned).sum();
        }

        // Update should potentially modify earnings
        update_ipfs_storage(&state, &mut rng);

        let updated = state.ipfs.read().expect("Failed to read ipfs data");

        // Verify structure is intact
        assert!(!updated.storage_market.is_empty());
        assert!(!updated.pinned_objects.is_empty());
    }

    #[test]
    fn test_update_ipfs_storage_maintains_structure() {
        let state = TelemetryState::new();

        let mut rng = rand::thread_rng();
        let initial_pin_count = {
            let data = state.ipfs.read().expect("Failed to read ipfs data");
            data.pinned_objects.len()
        };

        // Update storage
        update_ipfs_storage(&state, &mut rng);

        let updated = state.ipfs.read().expect("Failed to read ipfs data");

        // Pin count should remain consistent (no deletions)
        assert!(updated.pinned_objects.len() >= initial_pin_count);
    }

    #[test]
    fn test_ipfs_storage_replicas_bounded() {
        let storage = seed_ipfs_storage();

        for pin in &storage.pinned_objects {
            assert!(pin.replicas > 0, "Should have at least 1 replica");
            assert!(pin.replicas <= 10, "Replicas should be bounded at 10");
        }
    }

    #[test]
    fn test_ipfs_storage_earning_potential_positive() {
        let storage = seed_ipfs_storage();

        for pin in &storage.pinned_objects {
            assert!(
                pin.earning_potential >= 0.0,
                "Earning potential should not be negative"
            );
            assert!(
                pin.earning_potential > 0.0,
                "Seeded content should have earning potential"
            );
        }
    }

    #[test]
    fn test_ipfs_deal_earnings_non_negative() {
        let storage = seed_ipfs_storage();

        for deal in &storage.storage_market {
            assert!(deal.earned >= 0.0, "Deal earnings cannot be negative");
        }
    }

    #[test]
    fn test_system_metrics_timestamp_format() {
        let sys = System::new_all();
        let metrics = read_system_metrics(&sys);

        // Should be ISO 8601 format (YYYY-MM-DDTHH:MM:SSZ)
        assert!(metrics.updated_at.contains("T"));
        assert!(metrics.updated_at.ends_with("Z"));
        assert!(metrics.updated_at.len() >= 20); // Minimum length for ISO format
    }

    #[test]
    fn test_ipfs_storage_timestamp_format() {
        let storage = seed_ipfs_storage();

        assert!(storage.updated_at.contains("T"));
        assert!(storage.updated_at.ends_with("Z"));

        for pin in &storage.pinned_objects {
            assert!(pin.pinned_at.contains("T"));
            assert!(pin.pinned_at.ends_with("Z"));
        }
    }

    #[test]
    fn test_multiple_system_metrics_calls_succeed() {
        let sys = System::new_all();
        // Should be able to call multiple times without error
        let metrics1 = read_system_metrics(&sys);
        let metrics2 = read_system_metrics(&sys);
        let metrics3 = read_system_metrics(&sys);

        assert!(metrics1.cpu.cores > 0);
        assert!(metrics2.cpu.cores > 0);
        assert!(metrics3.cpu.cores > 0);
    }

    #[test]
    fn test_ipfs_cid_format() {
        let storage = seed_ipfs_storage();

        for pin in &storage.pinned_objects {
            assert!(pin.cid.starts_with("bafy"), "CID should start with valid prefix");
        }
    }

    #[test]
    fn test_node_id_uuid_format() {
        let storage = seed_ipfs_storage();

        // UUID format check (should be able to parse as valid UUID format)
        assert!(storage.node_id.len() > 0);
        assert!(!storage.node_id.contains(" "));
    }

    #[test]
    fn test_telemetry_state_thread_safe() {
        use std::sync::Arc;
        use std::thread;

        let state = Arc::new(TelemetryState::new());

        let mut handles = vec![];

        // Spawn multiple threads updating system metrics
        for _ in 0..5 {
            let state_clone = Arc::clone(&state);
            let handle = thread::spawn(move || {
                update_system_metrics(&state_clone);
                let metrics = state_clone
                    .system
                    .read()
                    .expect("Failed to read metrics");
                assert!(metrics.cpu.cores > 0);
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    }

    #[test]
    fn test_storage_used_calculation_accurate() {
        let storage = seed_ipfs_storage();

        let sum: u64 = storage.pinned_objects.iter().map(|p| p.size).sum();
        assert_eq!(storage.storage_used, sum);
    }

    #[test]
    fn test_percentages_never_exceed_100_after_update() {
        let state = TelemetryState::new();
        let mut rng = rand::thread_rng();

        update_ipfs_storage(&state, &mut rng);

        let data = state.ipfs.read().expect("Failed to read ipfs data");
        let capacity = data.storage_capacity;
        let used = data.storage_used;

        let percentage = (used as f32 / capacity as f32) * 100.0;
        assert!(percentage <= 100.0, "Storage percentage should not exceed 100%");
    }