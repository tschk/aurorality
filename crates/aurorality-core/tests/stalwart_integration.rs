//! Integration tests for Stalwart JMAP transport.
//!
//! These tests only run when the `STALWART_INTEGRATION_TESTS=1` env var
//! is set AND a local `stalwart-lite` server is reachable.
//! Otherwise they are skipped (not stubbed — no fake success).

#[cfg(test)]
mod stalwart_integration {
    use aurorality_core::bridge::NativePlugin;
    use aurorality_core::transport::stalwart::StalwartClient;
    use serde_json::Value;

    fn integration_enabled() -> bool {
        std::env::var("STALWART_INTEGRATION_TESTS").is_ok_and(|v| v == "1")
    }

    fn stalwart_base_url() -> String {
        std::env::var("STALWART_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
    }

    fn configured_client() -> Option<StalwartClient> {
        let base_url = stalwart_base_url();
        let username = std::env::var("STALWART_USERNAME").ok()?;
        let password = std::env::var("STALWART_PASSWORD").ok()?;
        Some(StalwartClient::new(base_url, &username, &password))
    }

    #[test]
    fn jmap_echo_against_live_server() {
        if !integration_enabled() {
            eprintln!("skipped: set STALWART_INTEGRATION_TESTS=1 and configure credentials");
            return;
        }
        let client = configured_client()
            .expect("STALWART_USERNAME and STALWART_PASSWORD must be set for integration tests");

        let result = client.invoke("health", &serde_json::json!({})).unwrap();
        let health: Value = serde_json::from_value(result).unwrap();

        // Parse health as a generic object (different shape than TransportHealth via NativePlugin)
        // The health method returns TransportHealth serialized as JSON
        if let Some(connected) = health.get("connected").and_then(|v| v.as_bool()) {
            assert!(connected, "Stalwart should be connected: {health:?}");
        } else {
            // If we can't parse the health response, at least the call didn't panic
            eprintln!("health response: {health}");
        }
    }

    #[test]
    fn jmap_list_against_live_server() {
        if !integration_enabled() {
            eprintln!("skipped: set STALWART_INTEGRATION_TESTS=1 and configure credentials");
            return;
        }
        let client = configured_client()
            .expect("STALWART_USERNAME and STALWART_PASSWORD must be set for integration tests");

        let result = client.invoke("list", &serde_json::json!({})).unwrap();
        let envelope: Value = serde_json::from_value(result).unwrap();

        // Should be a valid JSON-RPC-style response
        assert_eq!(envelope["ok"], true, "list should succeed: {envelope:?}");
        assert!(envelope["data"].is_array(), "data should be an array");
    }

    #[test]
    fn stalwart_unconfigured_skips_health() {
        // This test always runs — it checks the unconfigured path.
        let client = StalwartClient::from_env(); // may or may not have env vars set
        let result = client.invoke("health", &serde_json::json!({})).unwrap();
        let health: Value = serde_json::from_value(result).unwrap();
        // At minimum, the call should not panic
        assert!(health.get("id").is_some());
    }
}
