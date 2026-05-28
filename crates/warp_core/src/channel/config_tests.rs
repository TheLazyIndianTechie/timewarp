use super::*;
use crate::AppId;

#[test]
fn debug_redacts_sensitive_looking_channel_config_values() {
    let config = ChannelConfig {
        app_id: AppId::new("dev", "warp", "Warp"),
        logfile_name: "warp.log".into(),
        server_config: WarpServerConfig {
            server_root_url: "https://app.warp.dev".into(),
            rtc_server_url: "wss://rtc.app.warp.dev/graphql/v2".into(),
            session_sharing_server_url: Some("wss://sessions.app.warp.dev".into()),
            firebase_auth_api_key: "firebase-secret-looking-value".into(),
        },
        oz_config: OzConfig::production(),
        telemetry_config: Some(TelemetryConfig {
            telemetry_file_name: "telemetry.json".into(),
            rudderstack_config: Some(RudderStackConfig {
                write_key: "rudder-write-key".into(),
                root_url: "https://rudder.example.com".into(),
                ugc_write_key: "rudder-ugc-key".into(),
            }),
        }),
        autoupdate_config: None,
        crash_reporting_config: Some(CrashReportingConfig {
            sentry_url: "https://sentry-secret@example.com/1".into(),
        }),
        mcp_static_config: Some(McpStaticConfig {
            providers: vec![McpOAuthProviderConfig {
                issuer: "https://github.com/login/oauth".into(),
                client_id: "github-client-id".into(),
                client_secret: "github-client-secret".into(),
            }],
        }),
    };

    let debug = format!("{config:?}");

    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("firebase-secret-looking-value"));
    assert!(!debug.contains("rudder-write-key"));
    assert!(!debug.contains("rudder-ugc-key"));
    assert!(!debug.contains("sentry-secret"));
    assert!(!debug.contains("github-client-secret"));
    assert!(debug.contains("github-client-id"));
}
