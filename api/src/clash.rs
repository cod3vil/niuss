use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::models::Node;

/// Clash proxy configuration
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClashProxy {
    #[serde(rename = "ss")]
    Shadowsocks {
        name: String,
        server: String,
        port: u16,
        cipher: String,
        password: String,
        udp: bool,
    },
    #[serde(rename = "vmess")]
    VMess {
        name: String,
        server: String,
        port: u16,
        uuid: String,
        #[serde(rename = "alterId")]
        alter_id: u16,
        cipher: String,
        udp: bool,
        network: String,
    },
    #[serde(rename = "trojan")]
    Trojan {
        name: String,
        server: String,
        port: u16,
        password: String,
        udp: bool,
        sni: Option<String>,
        #[serde(rename = "skip-cert-verify")]
        skip_cert_verify: bool,
    },
    #[serde(rename = "hysteria2")]
    Hysteria2 {
        name: String,
        server: String,
        port: u16,
        password: String,
        obfs: Option<String>,
        #[serde(rename = "obfs-password")]
        obfs_password: Option<String>,
        sni: Option<String>,
        #[serde(rename = "skip-cert-verify")]
        skip_cert_verify: bool,
    },
    #[serde(rename = "vless")]
    VLESS {
        name: String,
        server: String,
        port: u16,
        uuid: String,
        flow: Option<String>,
        network: String,
        #[serde(rename = "reality-opts")]
        reality_opts: Option<RealityOpts>,
        #[serde(rename = "client-fingerprint")]
        client_fingerprint: Option<String>,
    },
}

/// Reality options for VLESS
#[derive(Debug, Serialize, Deserialize)]
pub struct RealityOpts {
    #[serde(rename = "public-key")]
    pub public_key: String,
    #[serde(rename = "short-id")]
    pub short_id: String,
}

/// Complete Clash configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct ClashConfig {
    pub proxies: Vec<ClashProxy>,
    #[serde(rename = "proxy-groups")]
    pub proxy_groups: Vec<ProxyGroup>,
    pub rules: Vec<String>,
}

/// Proxy group configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyGroup {
    pub name: String,
    #[serde(rename = "type")]
    pub group_type: String,
    pub proxies: Vec<String>,
}

/// Generate Clash configuration from nodes
pub fn generate_clash_config(nodes: &[Node]) -> Result<String> {
    let mut proxies = Vec::new();
    let mut proxy_names = Vec::new();

    for node in nodes {
        if let Some(proxy) = node_to_clash_proxy(node) {
            let name = get_proxy_name(&proxy);
            proxy_names.push(name);
            proxies.push(proxy);
        }
    }

    // Create proxy groups
    let proxy_groups = vec![
        ProxyGroup {
            name: "Proxy".to_string(),
            group_type: "select".to_string(),
            proxies: proxy_names.clone(),
        },
        ProxyGroup {
            name: "Auto".to_string(),
            group_type: "url-test".to_string(),
            proxies: proxy_names,
        },
    ];

    // Default rules
    let rules = vec![
        "DOMAIN-SUFFIX,google.com,Proxy".to_string(),
        "DOMAIN-SUFFIX,youtube.com,Proxy".to_string(),
        "DOMAIN-SUFFIX,facebook.com,Proxy".to_string(),
        "DOMAIN-SUFFIX,twitter.com,Proxy".to_string(),
        "GEOIP,CN,DIRECT".to_string(),
        "MATCH,Proxy".to_string(),
    ];

    let config = ClashConfig {
        proxies,
        proxy_groups,
        rules,
    };

    // Serialize to YAML
    let yaml = serde_yaml::to_string(&config)
        .map_err(|e| anyhow!("Failed to serialize Clash config: {}", e))?;

    Ok(yaml)
}

/// Generate Clash configuration from nodes with include_in_clash=true
/// This is the new unified approach that reads from the nodes table
/// and respects the include_in_clash and sort_order fields
pub async fn generate_clash_config_from_nodes(
    pool: &sqlx::PgPool,
) -> Result<String> {
    use crate::db::list_clash_nodes;
    
    // Query nodes with include_in_clash=true ordered by sort_order
    let nodes = list_clash_nodes(pool).await?;
    
    // Use the existing generate_clash_config function
    generate_clash_config(&nodes)
}

/// Generate Clash configuration from database models
pub fn generate_clash_config_from_db(
    db_proxies: &[crate::models::ClashProxy],
    db_proxy_groups: &[crate::models::ClashProxyGroup],
    db_rules: &[crate::models::ClashRule],
) -> Result<String> {
    // Convert database proxies to Clash proxies
    let mut proxies = Vec::new();
    for db_proxy in db_proxies {
        let proxy = db_proxy_to_clash_proxy(db_proxy)?;
        proxies.push(proxy);
    }

    // Convert database proxy groups to Clash proxy groups
    let proxy_groups: Vec<ProxyGroup> = db_proxy_groups
        .iter()
        .map(|g| ProxyGroup {
            name: g.name.clone(),
            group_type: g.group_type.clone(),
            proxies: g.proxies.clone(),
        })
        .collect();

    // Convert database rules to Clash rules
    let rules: Vec<String> = db_rules
        .iter()
        .map(|r| {
            let mut rule = format!("{}", r.rule_type);
            if let Some(ref value) = r.rule_value {
                rule.push_str(&format!(",{}", value));
            }
            rule.push_str(&format!(",{}", r.proxy_group));
            if r.no_resolve {
                rule.push_str(",no-resolve");
            }
            rule
        })
        .collect();

    let config = ClashConfig {
        proxies,
        proxy_groups,
        rules,
    };

    // Serialize to YAML
    let yaml = serde_yaml::to_string(&config)
        .map_err(|e| anyhow!("Failed to serialize Clash config: {}", e))?;

    Ok(yaml)
}

/// Convert database ClashProxy to Clash proxy enum
fn db_proxy_to_clash_proxy(db_proxy: &crate::models::ClashProxy) -> Result<ClashProxy> {
    match db_proxy.proxy_type.as_str() {
        "ss" => {
            let cipher = db_proxy.config
                .get("cipher")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'cipher' in Shadowsocks config"))?
                .to_string();
            
            let password = db_proxy.config
                .get("password")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'password' in Shadowsocks config"))?
                .to_string();

            Ok(ClashProxy::Shadowsocks {
                name: db_proxy.name.clone(),
                server: db_proxy.server.clone(),
                port: db_proxy.port as u16,
                cipher,
                password,
                udp: db_proxy.config.get("udp").and_then(|v| v.as_bool()).unwrap_or(true),
            })
        }
        "vmess" => {
            let uuid = db_proxy.config
                .get("uuid")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'uuid' in VMess config"))?
                .to_string();
            
            let alter_id = db_proxy.config
                .get("alterId")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u16;
            
            let cipher = db_proxy.config
                .get("cipher")
                .and_then(|v| v.as_str())
                .unwrap_or("auto")
                .to_string();
            
            let network = db_proxy.config
                .get("network")
                .and_then(|v| v.as_str())
                .unwrap_or("tcp")
                .to_string();

            Ok(ClashProxy::VMess {
                name: db_proxy.name.clone(),
                server: db_proxy.server.clone(),
                port: db_proxy.port as u16,
                uuid,
                alter_id,
                cipher,
                udp: db_proxy.config.get("udp").and_then(|v| v.as_bool()).unwrap_or(true),
                network,
            })
        }
        "trojan" => {
            let password = db_proxy.config
                .get("password")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'password' in Trojan config"))?
                .to_string();
            
            let sni = db_proxy.config
                .get("sni")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            
            let skip_cert_verify = db_proxy.config
                .get("skip-cert-verify")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            Ok(ClashProxy::Trojan {
                name: db_proxy.name.clone(),
                server: db_proxy.server.clone(),
                port: db_proxy.port as u16,
                password,
                udp: db_proxy.config.get("udp").and_then(|v| v.as_bool()).unwrap_or(true),
                sni,
                skip_cert_verify,
            })
        }
        "hysteria2" => {
            let password = db_proxy.config
                .get("password")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'password' in Hysteria2 config"))?
                .to_string();
            
            let obfs = db_proxy.config
                .get("obfs")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            
            let obfs_password = db_proxy.config
                .get("obfs-password")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            
            let sni = db_proxy.config
                .get("sni")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            
            let skip_cert_verify = db_proxy.config
                .get("skip-cert-verify")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            Ok(ClashProxy::Hysteria2 {
                name: db_proxy.name.clone(),
                server: db_proxy.server.clone(),
                port: db_proxy.port as u16,
                password,
                obfs,
                obfs_password,
                sni,
                skip_cert_verify,
            })
        }
        "vless" => {
            let uuid = db_proxy.config
                .get("uuid")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'uuid' in VLESS config"))?
                .to_string();
            
            let flow = db_proxy.config
                .get("flow")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            
            let network = db_proxy.config
                .get("network")
                .and_then(|v| v.as_str())
                .unwrap_or("tcp")
                .to_string();
            
            let reality_opts = if let Some(reality) = db_proxy.config.get("reality-opts") {
                let public_key = reality
                    .get("public-key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'public-key' in Reality config"))?
                    .to_string();
                
                let short_id = reality
                    .get("short-id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                Some(RealityOpts {
                    public_key,
                    short_id,
                })
            } else {
                None
            };
            
            let client_fingerprint = db_proxy.config
                .get("client-fingerprint")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| Some("chrome".to_string()));

            Ok(ClashProxy::VLESS {
                name: db_proxy.name.clone(),
                server: db_proxy.server.clone(),
                port: db_proxy.port as u16,
                uuid,
                flow,
                network,
                reality_opts,
                client_fingerprint,
            })
        }
        _ => Err(anyhow!("Unsupported proxy type: {}", db_proxy.proxy_type)),
    }
}

/// Map node protocol to Clash proxy type
/// Returns None for unsupported protocols and logs a warning
pub fn map_node_protocol_to_clash(protocol: &str) -> Option<&'static str> {
    match protocol {
        "shadowsocks" => Some("ss"),
        "vmess" => Some("vmess"),
        "trojan" => Some("trojan"),
        "hysteria2" => Some("hysteria2"),
        "vless" => Some("vless"),
        _ => {
            warn!("Unsupported protocol: {}", protocol);
            None
        }
    }
}

/// Merge node.secret and node.config into a complete configuration
/// Maps secret to password (ss/trojan/hysteria2) or uuid (vmess/vless) based on protocol
pub fn merge_node_config(node: &Node) -> serde_json::Value {
    let mut config = node.config.clone();
    
    // Add protocol-specific fields from node.secret
    match node.protocol.as_str() {
        "shadowsocks" => {
            // Only set password if not already in config
            if !config.get("password").is_some() {
                config["password"] = serde_json::json!(node.secret);
            }
        },
        "vmess" => {
            // Only set uuid if not already in config
            if !config.get("uuid").is_some() {
                config["uuid"] = serde_json::json!(node.secret);
            }
        },
        "trojan" => {
            // Only set password if not already in config
            if !config.get("password").is_some() {
                config["password"] = serde_json::json!(node.secret);
            }
        },
        "hysteria2" => {
            // Only set password if not already in config
            if !config.get("password").is_some() {
                config["password"] = serde_json::json!(node.secret);
            }
        },
        "vless" => {
            // Only set uuid if not already in config
            if !config.get("uuid").is_some() {
                config["uuid"] = serde_json::json!(node.secret);
            }
        },
        _ => {},
    }
    
    config
}

/// Convert a Node to a ClashProxy using the new unified approach
/// Maps node fields to Clash proxy format:
/// - node.name → proxy.name
/// - node.host → proxy.server
/// - node.port → proxy.port
/// - node.protocol → proxy.type (via map_node_protocol_to_clash)
/// - node.secret + node.config → proxy configuration (via merge_node_config)
pub fn node_to_clash_proxy(node: &Node) -> Option<ClashProxy> {
    // Check if protocol is supported
    map_node_protocol_to_clash(&node.protocol)?;
    
    // Merge secret and config
    let merged_config = merge_node_config(node);
    
    // Create a temporary node with merged config for the existing generator functions
    let mut temp_node = node.clone();
    temp_node.config = merged_config;
    
    // Use existing protocol-specific generators
    match node.protocol.as_str() {
        "shadowsocks" => generate_shadowsocks_proxy(&temp_node).ok(),
        "vmess" => generate_vmess_proxy(&temp_node).ok(),
        "trojan" => generate_trojan_proxy(&temp_node).ok(),
        "hysteria2" => generate_hysteria2_proxy(&temp_node).ok(),
        "vless" => generate_vless_proxy(&temp_node).ok(),
        _ => None,
    }
}

/// Convert a Node to a ClashProxy (legacy version for backward compatibility)
fn node_to_clash_proxy_legacy(node: &Node) -> Result<ClashProxy> {
    match node.protocol.as_str() {
        "shadowsocks" => generate_shadowsocks_proxy(node),
        "vmess" => generate_vmess_proxy(node),
        "trojan" => generate_trojan_proxy(node),
        "hysteria2" => generate_hysteria2_proxy(node),
        "vless" => generate_vless_proxy(node),
        _ => Err(anyhow!("Unsupported protocol: {}", node.protocol)),
    }
}
fn get_proxy_name(proxy: &ClashProxy) -> String {
    match proxy {
        ClashProxy::Shadowsocks { name, .. } => name.clone(),
        ClashProxy::VMess { name, .. } => name.clone(),
        ClashProxy::Trojan { name, .. } => name.clone(),
        ClashProxy::Hysteria2 { name, .. } => name.clone(),
        ClashProxy::VLESS { name, .. } => name.clone(),
    }
}

/// Generate Shadowsocks proxy configuration
fn generate_shadowsocks_proxy(node: &Node) -> Result<ClashProxy> {
    let config = &node.config;
    
    let cipher = config
        .get("method")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing 'method' in Shadowsocks config"))?
        .to_string();
    
    let password = config
        .get("password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing 'password' in Shadowsocks config"))?
        .to_string();

    Ok(ClashProxy::Shadowsocks {
        name: node.name.clone(),
        server: node.host.clone(),
        port: node.port as u16,
        cipher,
        password,
        udp: true,
    })
}

/// Generate VMess proxy configuration
fn generate_vmess_proxy(node: &Node) -> Result<ClashProxy> {
    let config = &node.config;
    
    let uuid = config
        .get("uuid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing 'uuid' in VMess config"))?
        .to_string();
    
    let alter_id = config
        .get("alter_id")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u16;
    
    let security = config
        .get("security")
        .and_then(|v| v.as_str())
        .unwrap_or("auto")
        .to_string();
    
    let network = config
        .get("network")
        .and_then(|v| v.as_str())
        .unwrap_or("tcp")
        .to_string();

    Ok(ClashProxy::VMess {
        name: node.name.clone(),
        server: node.host.clone(),
        port: node.port as u16,
        uuid,
        alter_id,
        cipher: security,
        udp: true,
        network,
    })
}

/// Generate Trojan proxy configuration
fn generate_trojan_proxy(node: &Node) -> Result<ClashProxy> {
    let config = &node.config;
    
    let password = config
        .get("password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing 'password' in Trojan config"))?
        .to_string();
    
    let sni = config
        .get("sni")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    
    let skip_cert_verify = config
        .get("skip_cert_verify")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok(ClashProxy::Trojan {
        name: node.name.clone(),
        server: node.host.clone(),
        port: node.port as u16,
        password,
        udp: true,
        sni,
        skip_cert_verify,
    })
}

/// Generate Hysteria2 proxy configuration
fn generate_hysteria2_proxy(node: &Node) -> Result<ClashProxy> {
    let config = &node.config;
    
    let password = config
        .get("password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing 'password' in Hysteria2 config"))?
        .to_string();
    
    let obfs = config
        .get("obfs")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    
    let obfs_password = config
        .get("obfs_password")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    
    let sni = config
        .get("sni")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    
    let skip_cert_verify = config
        .get("skip_cert_verify")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok(ClashProxy::Hysteria2 {
        name: node.name.clone(),
        server: node.host.clone(),
        port: node.port as u16,
        password,
        obfs,
        obfs_password,
        sni,
        skip_cert_verify,
    })
}

/// Generate VLESS proxy configuration
fn generate_vless_proxy(node: &Node) -> Result<ClashProxy> {
    let config = &node.config;
    
    let uuid = config
        .get("uuid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing 'uuid' in VLESS config"))?
        .to_string();
    
    let flow = config
        .get("flow")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    
    let network = config
        .get("network")
        .and_then(|v| v.as_str())
        .unwrap_or("tcp")
        .to_string();
    
    // Parse Reality configuration if present
    let reality_opts = if let Some(reality) = config.get("reality") {
        let public_key = reality
            .get("publicKey")
            .or_else(|| reality.get("public_key"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'publicKey' in Reality config"))?
            .to_string();
        
        let short_id = reality
            .get("shortIds")
            .or_else(|| reality.get("short_ids"))
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        Some(RealityOpts {
            public_key,
            short_id,
        })
    } else {
        None
    };
    
    let client_fingerprint = config
        .get("client_fingerprint")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| Some("chrome".to_string()));

    Ok(ClashProxy::VLESS {
        name: node.name.clone(),
        server: node.host.clone(),
        port: node.port as u16,
        uuid,
        flow,
        network,
        reality_opts,
        client_fingerprint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::Value as JsonValue;

    fn create_test_node(protocol: &str, config: JsonValue) -> Node {
        Node {
            id: 1,
            name: format!("Test {} Node", protocol),
            host: "example.com".to_string(),
            port: 443,
            protocol: protocol.to_string(),
            secret: "test_secret".to_string(),
            config,
            status: "online".to_string(),
            max_users: 1000,
            current_users: 0,
            total_upload: 0,
            total_download: 0,
            last_heartbeat: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            include_in_clash: true,
            sort_order: 0,
        }
    }

    #[test]
    fn test_generate_shadowsocks_proxy() {
        let config = serde_json::json!({
            "method": "aes-256-gcm",
            "password": "test_password"
        });
        
        let node = create_test_node("shadowsocks", config);
        let proxy = generate_shadowsocks_proxy(&node).unwrap();
        
        match proxy {
            ClashProxy::Shadowsocks { name, server, port, cipher, password, udp } => {
                assert_eq!(name, "Test shadowsocks Node");
                assert_eq!(server, "example.com");
                assert_eq!(port, 443);
                assert_eq!(cipher, "aes-256-gcm");
                assert_eq!(password, "test_password");
                assert!(udp);
            }
            _ => panic!("Expected Shadowsocks proxy"),
        }
    }

    #[test]
    fn test_generate_vmess_proxy() {
        let config = serde_json::json!({
            "uuid": "12345678-1234-1234-1234-123456789012",
            "alter_id": 0,
            "security": "auto",
            "network": "tcp"
        });
        
        let node = create_test_node("vmess", config);
        let proxy = generate_vmess_proxy(&node).unwrap();
        
        match proxy {
            ClashProxy::VMess { name, server, port, uuid, alter_id, cipher, udp, network } => {
                assert_eq!(name, "Test vmess Node");
                assert_eq!(server, "example.com");
                assert_eq!(port, 443);
                assert_eq!(uuid, "12345678-1234-1234-1234-123456789012");
                assert_eq!(alter_id, 0);
                assert_eq!(cipher, "auto");
                assert!(udp);
                assert_eq!(network, "tcp");
            }
            _ => panic!("Expected VMess proxy"),
        }
    }

    #[test]
    fn test_generate_trojan_proxy() {
        let config = serde_json::json!({
            "password": "test_password",
            "sni": "example.com",
            "skip_cert_verify": false
        });
        
        let node = create_test_node("trojan", config);
        let proxy = generate_trojan_proxy(&node).unwrap();
        
        match proxy {
            ClashProxy::Trojan { name, server, port, password, udp, sni, skip_cert_verify } => {
                assert_eq!(name, "Test trojan Node");
                assert_eq!(server, "example.com");
                assert_eq!(port, 443);
                assert_eq!(password, "test_password");
                assert!(udp);
                assert_eq!(sni, Some("example.com".to_string()));
                assert!(!skip_cert_verify);
            }
            _ => panic!("Expected Trojan proxy"),
        }
    }

    #[test]
    fn test_generate_hysteria2_proxy() {
        let config = serde_json::json!({
            "password": "test_password",
            "obfs": "salamander",
            "obfs_password": "obfs_pass",
            "sni": "example.com",
            "skip_cert_verify": false
        });
        
        let node = create_test_node("hysteria2", config);
        let proxy = generate_hysteria2_proxy(&node).unwrap();
        
        match proxy {
            ClashProxy::Hysteria2 { name, server, port, password, obfs, obfs_password, sni, skip_cert_verify } => {
                assert_eq!(name, "Test hysteria2 Node");
                assert_eq!(server, "example.com");
                assert_eq!(port, 443);
                assert_eq!(password, "test_password");
                assert_eq!(obfs, Some("salamander".to_string()));
                assert_eq!(obfs_password, Some("obfs_pass".to_string()));
                assert_eq!(sni, Some("example.com".to_string()));
                assert!(!skip_cert_verify);
            }
            _ => panic!("Expected Hysteria2 proxy"),
        }
    }

    #[test]
    fn test_generate_vless_proxy() {
        let config = serde_json::json!({
            "uuid": "12345678-1234-1234-1234-123456789012",
            "flow": "xtls-rprx-vision",
            "network": "tcp",
            "reality": {
                "publicKey": "test_public_key",
                "shortIds": [""]
            }
        });
        
        let node = create_test_node("vless", config);
        let proxy = generate_vless_proxy(&node).unwrap();
        
        match proxy {
            ClashProxy::VLESS { name, server, port, uuid, flow, network, reality_opts, client_fingerprint } => {
                assert_eq!(name, "Test vless Node");
                assert_eq!(server, "example.com");
                assert_eq!(port, 443);
                assert_eq!(uuid, "12345678-1234-1234-1234-123456789012");
                assert_eq!(flow, Some("xtls-rprx-vision".to_string()));
                assert_eq!(network, "tcp");
                assert!(reality_opts.is_some());
                if let Some(reality) = reality_opts {
                    assert_eq!(reality.public_key, "test_public_key");
                    assert_eq!(reality.short_id, "");
                }
                assert_eq!(client_fingerprint, Some("chrome".to_string()));
            }
            _ => panic!("Expected VLESS proxy"),
        }
    }

    #[test]
    fn test_generate_clash_config() {
        let nodes = vec![
            create_test_node("shadowsocks", serde_json::json!({
                "method": "aes-256-gcm",
                "password": "test_password"
            })),
            create_test_node("vmess", serde_json::json!({
                "uuid": "12345678-1234-1234-1234-123456789012",
                "alter_id": 0,
                "security": "auto"
            })),
        ];
        
        let yaml = generate_clash_config(&nodes).unwrap();
        
        // Verify YAML is valid
        assert!(yaml.contains("proxies:"));
        assert!(yaml.contains("proxy-groups:"));
        assert!(yaml.contains("rules:"));
        assert!(yaml.contains("Test shadowsocks Node"));
        assert!(yaml.contains("Test vmess Node"));
    }

    #[test]
    fn test_reality_config_completeness() {
        let config = serde_json::json!({
            "uuid": "12345678-1234-1234-1234-123456789012",
            "flow": "xtls-rprx-vision",
            "network": "tcp",
            "reality": {
                "publicKey": "test_public_key_12345",
                "privateKey": "test_private_key_67890",
                "shortIds": ["abc123", "def456"],
                "dest": "www.microsoft.com:443",
                "serverNames": ["www.microsoft.com"]
            }
        });
        
        let node = create_test_node("vless", config);
        let proxy = generate_vless_proxy(&node).unwrap();
        
        match proxy {
            ClashProxy::VLESS { reality_opts, .. } => {
                assert!(reality_opts.is_some());
                let reality = reality_opts.unwrap();
                assert_eq!(reality.public_key, "test_public_key_12345");
                assert_eq!(reality.short_id, "abc123"); // First short ID
            }
            _ => panic!("Expected VLESS proxy"),
        }
    }
}
