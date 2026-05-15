use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use crate::hue_tls_pin::build_hue_http_client;
use crate::{HueBridgeConfig, HueError, Result};

#[derive(Debug, Clone)]
pub struct HueClient {
    bridge_ip: String,
    app_key: Option<String>,
    http: reqwest::Client,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BridgeUser {
    pub app_key: String,
    pub client_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EntertainmentArea {
    pub id: String,
    pub name: String,
    pub channels: Vec<EntertainmentChannel>,
    /// Nombre de lumières uniques rattachées à la zone (rempli par `entertainment_areas_with_light_counts`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lights: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct BridgeInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EntertainmentChannel {
    pub channel_id: u8,
    pub position: Option<EntertainmentChannelPosition>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub struct EntertainmentChannelPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct BridgeDiscovery {
    pub id: String,
    #[serde(rename = "internalipaddress")]
    pub ip: String,
}

pub async fn discover_bridges() -> Result<Vec<BridgeDiscovery>> {
    let mut bridges = BTreeMap::new();

    for bridge in discover_bridges_via_cloud().await? {
        bridges.insert(bridge.ip.clone(), bridge);
    }
    for bridge in discover_bridges_via_ssdp().map_err(|err| HueError::Request(err.to_string()))? {
        bridges.insert(bridge.ip.clone(), bridge);
    }
    for bridge in discover_bridges_via_subnet_scan() {
        bridges.insert(bridge.ip.clone(), bridge);
    }

    Ok(bridges.into_values().collect())
}

async fn discover_bridges_via_cloud() -> Result<Vec<BridgeDiscovery>> {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|err| HueError::Request(err.to_string()))?;

    http.get("https://discovery.meethue.com")
        .send()
        .await
        .map_err(|err| HueError::Request(err.to_string()))?
        .json()
        .await
        .map_err(|err| HueError::Request(err.to_string()))
}

fn discover_bridges_via_ssdp() -> io::Result<Vec<BridgeDiscovery>> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_millis(350)))?;
    socket.set_multicast_loop_v4(false)?;

    for search_target in ["urn:schemas-upnp-org:device:basic:1", "ssdp:all"] {
        let request = format!(
            "M-SEARCH * HTTP/1.1\r\n\
             HOST: 239.255.255.250:1900\r\n\
             MAN: \"ssdp:discover\"\r\n\
             MX: 1\r\n\
             ST: {search_target}\r\n\
             \r\n"
        );
        socket.send_to(request.as_bytes(), "239.255.255.250:1900")?;
    }

    let deadline = Instant::now() + Duration::from_secs(2);
    let mut buf = [0_u8; 2048];
    let mut bridges = BTreeMap::new();
    while Instant::now() < deadline {
        match socket.recv_from(&mut buf) {
            Ok((len, _)) => {
                let response = String::from_utf8_lossy(&buf[..len]);
                if let Some(bridge) = parse_ssdp_bridge_response(&response) {
                    bridges.insert(bridge.ip.clone(), bridge);
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                break;
            }
            Err(error) => return Err(error),
        }
    }

    Ok(bridges.into_values().collect())
}

/// Local subnet scan for Hue bridges. Uses at most `LUMAWAY_SUBNET_SCAN_CONCURRENCY` concurrent TCP
/// probes (default 64, clamped to 1–256) to avoid spawning hundreds of threads on large /24 scans.
fn discover_bridges_via_subnet_scan() -> Vec<BridgeDiscovery> {
    let local_ips = local_ipv4_candidates();
    if local_ips.is_empty() {
        return Vec::new();
    }
    let mut candidates = Vec::new();
    for local_ip in local_ips {
        let octets = local_ip.octets();
        for host in 1..=254_u8 {
            if host == octets[3] {
                continue;
            }
            candidates.push(Ipv4Addr::new(octets[0], octets[1], octets[2], host));
        }
    }
    if candidates.is_empty() {
        return Vec::new();
    }

    let worker_count = std::env::var("LUMAWAY_SUBNET_SCAN_CONCURRENCY")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(64)
        .clamp(1, 256)
        .min(candidates.len());

    let next = AtomicUsize::new(0);
    let bridges = Mutex::new(BTreeMap::new());

    thread::scope(|scope| {
        for _ in 0..worker_count {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                if index >= candidates.len() {
                    break;
                }
                if let Some(bridge) = probe_bridge_config(candidates[index]) {
                    if let Ok(mut map) = bridges.lock() {
                        map.insert(bridge.ip.clone(), bridge);
                    }
                }
            });
        }
    });

    bridges
        .into_inner()
        .expect("subnet scan workers should not poison bridge map")
        .into_values()
        .collect()
}

fn local_ipv4_candidates() -> Vec<Ipv4Addr> {
    let mut ips = BTreeSet::new();
    if let Some(ip) = route_ipv4() {
        ips.insert(ip);
    }
    if let Ok(output) = Command::new("hostname").arg("-I").output() {
        let text = String::from_utf8_lossy(&output.stdout);
        for value in text.split_whitespace() {
            if let Ok(ip) = value.parse::<Ipv4Addr>() {
                if is_lan_candidate(ip) {
                    ips.insert(ip);
                }
            }
        }
    }
    ips.into_iter().collect()
}

fn route_ipv4() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(ip) if is_lan_candidate(ip) => Some(ip),
        _ => None,
    }
}

fn is_lan_candidate(ip: Ipv4Addr) -> bool {
    ip.is_private() && !ip.is_loopback()
}

fn probe_bridge_config(ip: Ipv4Addr) -> Option<BridgeDiscovery> {
    let addr = SocketAddr::new(IpAddr::V4(ip), 80);
    let timeout = Duration::from_millis(180);
    let mut stream = TcpStream::connect_timeout(&addr, timeout).ok()?;
    stream.set_read_timeout(Some(timeout)).ok()?;
    stream.set_write_timeout(Some(timeout)).ok()?;
    stream
        .write_all(b"GET /api/config HTTP/1.1\r\nHost: local\r\nConnection: close\r\n\r\n")
        .ok()?;

    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;
    parse_bridge_config_response(&response, &ip.to_string())
}

fn parse_bridge_config_response(response: &str, fallback_ip: &str) -> Option<BridgeDiscovery> {
    let body = response.split("\r\n\r\n").nth(1).unwrap_or(response);
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let id = value
        .get("bridgeid")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(fallback_ip)
        .to_string();
    let ip = value
        .get("ipaddress")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(fallback_ip)
        .to_string();
    let has_bridge_shape = value.get("bridgeid").is_some()
        || value
            .get("apiversion")
            .and_then(serde_json::Value::as_str)
            .is_some();
    has_bridge_shape.then_some(BridgeDiscovery { id, ip })
}

fn parse_ssdp_bridge_response(response: &str) -> Option<BridgeDiscovery> {
    let lower = response.to_ascii_lowercase();
    if !lower.contains("hue-bridgeid")
        && !lower.contains("ipbridge")
        && !lower.contains("philips hue")
    {
        return None;
    }

    let id = header_value(response, "hue-bridgeid").unwrap_or_default();
    let location = header_value(response, "location")?;
    let ip = host_from_url(&location)?;
    Some(BridgeDiscovery {
        id: if id.is_empty() { ip.clone() } else { id },
        ip,
    })
}

fn header_value(response: &str, name: &str) -> Option<String> {
    response.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.trim()
            .eq_ignore_ascii_case(name)
            .then(|| value.trim().to_string())
    })
}

fn host_from_url(url: &str) -> Option<String> {
    let without_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let host_port = without_scheme.split('/').next()?.trim();
    let host = host_port.split(':').next()?.trim();
    (!host.is_empty()).then(|| host.to_string())
}

impl HueClient {
    /// Builds a client for the configured bridge IP.
    ///
    /// # TLS and the Hue bridge
    ///
    /// Hue bridges use HTTPS with **self-signed** certificates. By default, reqwest is built with
    /// `ClientBuilder::danger_accept_invalid_certs(true)` so REST calls succeed without importing a
    /// custom root CA.
    ///
    /// Set `LUMAWAY_HUE_PIN_CERTS=1` to enable **TLS pinning** (default: SHA-256 of leaf **SPKI**;
    /// `LUMAWAY_HUE_PIN_MODE=cert` pins full leaf DER). Pins live under the Lumaway config directory
    /// on first successful TLS (see `docs/security.md`). Optional `LUMAWAY_BRIDGE_ID` and
    /// `by-id/` paths bind pins to the bridge hardware id after `promote_bridge_tls_pin`.
    ///
    /// **Trust model:** you are trusting that traffic to `bridge_ip` reaches the real bridge. On a
    /// compromised LAN, a MITM could impersonate the bridge. See the project `docs/security.md`
    /// file for a longer discussion.
    pub fn new(config: HueBridgeConfig) -> Result<Self> {
        if config.bridge_ip.trim().is_empty() {
            return Err(HueError::MissingBridgeIp);
        }

        let http = build_hue_http_client(config.bridge_ip.trim())?;

        Ok(Self {
            bridge_ip: config.bridge_ip,
            app_key: config.app_key,
            http,
        })
    }

    pub async fn create_user(&self, application_name: &str) -> Result<BridgeUser> {
        let url = format!("https://{}/api", self.bridge_ip);
        let payload = serde_json::json!({
            "devicetype": format!("{application_name}#user"),
            "generateclientkey": true,
        });

        let response: Vec<CreateUserResponse> = self
            .http
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(|err| HueError::Request(err.to_string()))?
            .json()
            .await
            .map_err(|err| HueError::Request(err.to_string()))?;

        let first = response
            .into_iter()
            .next()
            .ok_or_else(|| HueError::UnexpectedResponse("empty create user response".into()))?;

        match (first.success, first.error) {
            (Some(success), _) => Ok(BridgeUser {
                app_key: success.username,
                client_key: success.clientkey,
            }),
            (_, Some(error)) => Err(HueError::Bridge(error.description)),
            _ => Err(HueError::UnexpectedResponse(
                "create user response contained no success or error".into(),
            )),
        }
    }

    pub async fn entertainment_areas(&self) -> Result<Vec<EntertainmentArea>> {
        let response: ResourceResponse<EntertainmentConfigurationResource> = self
            .request("GET", "/resource/entertainment_configuration", None)
            .await?;

        response
            .data
            .into_iter()
            .map(EntertainmentArea::try_from)
            .collect::<Result<Vec<_>>>()
    }

    /// Comme [`Self::entertainment_areas`], mais compte les lumières Hue réellement liées à chaque zone.
    pub async fn entertainment_areas_with_light_counts(&self) -> Result<Vec<EntertainmentArea>> {
        let response: ResourceResponse<EntertainmentConfigurationResource> = self
            .request("GET", "/resource/entertainment_configuration", None)
            .await?;

        let mut areas = Vec::new();
        for item in response.data {
            let lights = self.light_ids_for_configuration(&item).await?.len();
            let mut area = EntertainmentArea::try_from(item)?;
            area.lights = Some(lights);
            areas.push(area);
        }
        Ok(areas)
    }

    pub async fn bridge_info(&self) -> Result<BridgeInfo> {
        let response: ResourceResponse<BridgeResource> =
            self.request("GET", "/resource/bridge", None).await?;

        let bridge = response
            .data
            .into_iter()
            .next()
            .ok_or_else(|| HueError::UnexpectedResponse("bridge resource not found".into()))?;
        let path = format!("/resource/device/{}", bridge.owner.rid);
        let response: ResourceResponse<DeviceResource> = self.request("GET", &path, None).await?;
        let device = response.data.into_iter().next().ok_or_else(|| {
            HueError::UnexpectedResponse(format!(
                "bridge owner device not found: {}",
                bridge.owner.rid
            ))
        })?;

        Ok(BridgeInfo {
            id: bridge.id,
            name: device.metadata.name,
        })
    }

    pub async fn entertainment_area(&self, area_id: &str) -> Result<EntertainmentArea> {
        let path = format!("/resource/entertainment_configuration/{area_id}");
        let response: ResourceResponse<EntertainmentConfigurationResource> =
            self.request("GET", &path, None).await?;

        let item = response.data.into_iter().next().ok_or_else(|| {
            HueError::UnexpectedResponse(format!("entertainment area not found: {area_id}"))
        })?;

        EntertainmentArea::try_from(item)
    }

    pub async fn resolve_entertainment_area(&self, area_ref: &str) -> Result<EntertainmentArea> {
        let matches: Vec<_> = self
            .entertainment_areas()
            .await?
            .into_iter()
            .filter(|area| area.id == area_ref || area.name == area_ref)
            .collect();

        let area_id = match matches.as_slice() {
            [area] => area.id.as_str(),
            [] => {
                return Err(HueError::UnexpectedResponse(format!(
                    "entertainment area not found by id or name: {area_ref}"
                )));
            }
            _ => {
                return Err(HueError::UnexpectedResponse(format!(
                    "multiple entertainment areas matched name: {area_ref}; use the area id instead"
                )));
            }
        };

        // Liste groupée : la réponse peut omettre `channels`; GET par id renvoie la disposition complète.
        self.entertainment_area(area_id).await
    }

    pub async fn entertainment_channel_ids(&self, area_id: &str) -> Result<Vec<u8>> {
        Ok(self
            .entertainment_area(area_id)
            .await?
            .channels
            .into_iter()
            .map(|channel| channel.channel_id)
            .collect())
    }

    pub async fn application_id(&self) -> Result<String> {
        let app_key = self.app_key.as_deref().ok_or(HueError::MissingAppKey)?;
        let url = format!("https://{}/auth/v1", self.bridge_ip);
        let response = self
            .http
            .get(url)
            .header("hue-application-key", app_key)
            .send()
            .await
            .map_err(|err| HueError::Request(err.to_string()))?;

        let status = response.status();
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(HueError::Authentication);
        }
        if !status.is_success() {
            return Err(HueError::HttpStatus(status.as_u16()));
        }

        response
            .headers()
            .get("hue-application-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string)
            .ok_or_else(|| HueError::UnexpectedResponse("missing hue-application-id header".into()))
    }

    pub async fn activate_entertainment(&self, area_id: &str) -> Result<()> {
        let path = format!("/resource/entertainment_configuration/{area_id}");
        self.request::<serde_json::Value>(
            "PUT",
            &path,
            Some(serde_json::json!({ "action": "start" })),
        )
        .await?;
        Ok(())
    }

    pub async fn deactivate_entertainment(&self, area_id: &str) -> Result<()> {
        let path = format!("/resource/entertainment_configuration/{area_id}");
        self.request::<serde_json::Value>(
            "PUT",
            &path,
            Some(serde_json::json!({ "action": "stop" })),
        )
        .await?;
        Ok(())
    }

    pub async fn set_entertainment_area_lights(
        &self,
        area_ref: &str,
        on: bool,
        brightness: Option<f64>,
    ) -> Result<usize> {
        let area = self.resolve_entertainment_area(area_ref).await?;
        let resource: ResourceResponse<EntertainmentConfigurationResource> = self
            .request(
                "GET",
                &format!("/resource/entertainment_configuration/{}", area.id),
                None,
            )
            .await?;
        let config = resource.data.into_iter().next().ok_or_else(|| {
            HueError::UnexpectedResponse(format!("entertainment area not found: {}", area.id))
        })?;

        let light_ids = self.light_ids_for_configuration(&config).await?;

        let brightness = brightness.map(|value| value.clamp(1.0, 100.0));
        for light_id in &light_ids {
            let body = if on {
                let mut body = serde_json::json!({ "on": { "on": true } });
                if let Some(brightness) = brightness {
                    body["dimming"] = serde_json::json!({ "brightness": brightness });
                }
                body
            } else {
                serde_json::json!({ "on": { "on": false } })
            };
            self.request::<serde_json::Value>(
                "PUT",
                &format!("/resource/light/{light_id}"),
                Some(body),
            )
            .await?;
        }

        Ok(light_ids.len())
    }

    async fn light_ids_for_configuration(
        &self,
        config: &EntertainmentConfigurationResource,
    ) -> Result<BTreeSet<String>> {
        let mut light_ids = BTreeSet::new();
        for channel in &config.channels {
            for member in &channel.members {
                if member.service.rtype != "entertainment" {
                    continue;
                }
                let entertainment: ResourceResponse<EntertainmentServiceResource> = self
                    .request(
                        "GET",
                        &format!("/resource/entertainment/{}", member.service.rid),
                        None,
                    )
                    .await?;
                for service in entertainment.data {
                    if service.renderer_reference.rtype == "light" {
                        light_ids.insert(service.renderer_reference.rid.clone());
                    }
                }
            }
        }
        Ok(light_ids)
    }

    async fn request<T>(
        &self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let app_key = self.app_key.as_deref().ok_or(HueError::MissingAppKey)?;
        let url = format!("https://{}/clip/v2{path}", self.bridge_ip);
        let method = method
            .parse::<reqwest::Method>()
            .map_err(|err| HueError::Request(err.to_string()))?;

        let mut request = self
            .http
            .request(method, url)
            .header("hue-application-key", app_key);

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request
            .send()
            .await
            .map_err(|err| HueError::Request(err.to_string()))?;

        let status = response.status();
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(HueError::Authentication);
        }

        if !status.is_success() {
            return Err(HueError::HttpStatus(status.as_u16()));
        }

        response
            .json()
            .await
            .map_err(|err| HueError::Request(err.to_string()))
    }
}

#[derive(Debug, Deserialize)]
struct CreateUserResponse {
    success: Option<CreateUserSuccess>,
    error: Option<CreateUserError>,
}

#[derive(Debug, Deserialize)]
struct CreateUserSuccess {
    username: String,
    clientkey: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateUserError {
    description: String,
}

#[derive(Debug, Deserialize)]
struct ResourceResponse<T> {
    data: Vec<T>,
}

#[derive(Debug, Deserialize)]
struct EntertainmentConfigurationResource {
    id: String,
    metadata: ResourceMetadata,
    #[serde(default)]
    channels: Vec<EntertainmentChannelResource>,
}

#[derive(Debug, Deserialize)]
struct BridgeResource {
    id: String,
    owner: ResourceReference,
}

#[derive(Debug, Deserialize)]
struct DeviceResource {
    metadata: ResourceMetadata,
}

#[derive(Debug, Deserialize)]
struct ResourceReference {
    rid: String,
    rtype: String,
}

#[derive(Debug, Deserialize)]
struct EntertainmentChannelResource {
    channel_id: u8,
    position: Option<EntertainmentChannelPosition>,
    #[serde(default)]
    members: Vec<EntertainmentChannelMemberResource>,
}

#[derive(Debug, Deserialize)]
struct EntertainmentChannelMemberResource {
    service: ResourceReference,
}

#[derive(Debug, Deserialize)]
struct EntertainmentServiceResource {
    renderer_reference: ResourceReference,
}

#[derive(Debug, Deserialize)]
struct ResourceMetadata {
    name: String,
}

impl TryFrom<EntertainmentConfigurationResource> for EntertainmentArea {
    type Error = HueError;

    fn try_from(value: EntertainmentConfigurationResource) -> Result<Self> {
        Ok(Self {
            id: value.id,
            name: value.metadata.name,
            channels: value
                .channels
                .into_iter()
                .map(|channel| EntertainmentChannel {
                    channel_id: channel.channel_id,
                    position: channel.position,
                })
                .collect(),
            lights: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse_bridge_config_response, parse_ssdp_bridge_response, BridgeInfo, BridgeResource,
        CreateUserResponse, DeviceResource, EntertainmentConfigurationResource, ResourceResponse,
    };

    #[test]
    fn parses_create_user_success() {
        let parsed: Vec<CreateUserResponse> = serde_json::from_str(
            r#"[{"success":{"username":"app-key","clientkey":"client-key"}}]"#,
        )
        .unwrap();

        let success = parsed[0].success.as_ref().unwrap();
        assert_eq!(success.username, "app-key");
        assert_eq!(success.clientkey.as_deref(), Some("client-key"));
    }

    #[test]
    fn parses_entertainment_configuration_list() {
        let parsed: ResourceResponse<EntertainmentConfigurationResource> = serde_json::from_str(
            r#"{
                    "data": [{
                        "id": "area-1",
                        "metadata": {"name": "TV"},
                        "channels": [
                            {"channel_id": 0, "position": {"x": -1.0, "y": 0.0, "z": 0.0}},
                            {"channel_id": 1, "position": {"x": 1.0, "y": 0.0, "z": 0.0}}
                        ]
                    }]
                }"#,
        )
        .unwrap();

        assert_eq!(parsed.data[0].id, "area-1");
        assert_eq!(parsed.data[0].metadata.name, "TV");
        assert_eq!(parsed.data[0].channels.len(), 2);
        assert_eq!(parsed.data[0].channels[0].channel_id, 0);
        assert_eq!(parsed.data[0].channels[0].position.unwrap().x, -1.0);
    }

    #[test]
    fn parses_bridge_resource() {
        let parsed: ResourceResponse<BridgeResource> = serde_json::from_str(
            r#"{
                    "data": [{
                        "id": "bridge-1",
                        "owner": {"rid": "device-1", "rtype": "device"}
                    }]
                }"#,
        )
        .unwrap();
        let device: ResourceResponse<DeviceResource> = serde_json::from_str(
            r#"{
                    "data": [{
                        "id": "device-1",
                        "metadata": {"name": "Salon Bridge"}
                    }]
                }"#,
        )
        .unwrap();

        let bridge = BridgeInfo {
            id: parsed.data[0].id.clone(),
            name: device.data[0].metadata.name.clone(),
        };

        assert_eq!(
            bridge,
            BridgeInfo {
                id: "bridge-1".to_string(),
                name: "Salon Bridge".to_string()
            }
        );
    }

    #[test]
    fn maps_entertainment_configuration_to_public_area() {
        let parsed: ResourceResponse<EntertainmentConfigurationResource> = serde_json::from_str(
            r#"{
                    "data": [{
                        "id": "area-1",
                        "metadata": {"name": "TV"},
                        "channels": [
                            {"channel_id": 3, "position": {"x": -0.5, "y": 1.0, "z": -0.2}},
                            {"channel_id": 4}
                        ]
                    }]
                }"#,
        )
        .unwrap();

        let area =
            super::EntertainmentArea::try_from(parsed.data.into_iter().next().unwrap()).unwrap();

        assert_eq!(area.id, "area-1");
        assert_eq!(area.name, "TV");
        assert_eq!(area.channels.len(), 2);
        assert_eq!(area.channels[0].channel_id, 3);
        assert_eq!(
            area.channels[0].position,
            Some(super::EntertainmentChannelPosition {
                x: -0.5,
                y: 1.0,
                z: -0.2
            })
        );
        assert_eq!(area.channels[1].channel_id, 4);
        assert_eq!(area.channels[1].position, None);
        assert_eq!(area.lights, None);
    }

    #[test]
    fn parses_ssdp_bridge_response() {
        let bridge = parse_ssdp_bridge_response(
            "HTTP/1.1 200 OK\r\n\
             LOCATION: http://192.168.1.108:80/description.xml\r\n\
             HUE-BRIDGEID: 001788FFFE123456\r\n\
             ST: urn:schemas-upnp-org:device:basic:1\r\n\
             \r\n",
        )
        .unwrap();

        assert_eq!(bridge.id, "001788FFFE123456");
        assert_eq!(bridge.ip, "192.168.1.108");
    }

    #[test]
    fn ignores_unrelated_ssdp_response() {
        assert!(parse_ssdp_bridge_response(
            "HTTP/1.1 200 OK\r\nLOCATION: http://192.168.1.1/root.xml\r\n\r\n"
        )
        .is_none());
    }

    #[test]
    fn parses_bridge_config_response() {
        let bridge = parse_bridge_config_response(
            "HTTP/1.1 200 OK\r\n\r\n{\"bridgeid\":\"001788FFFE123456\",\"ipaddress\":\"192.168.1.108\",\"apiversion\":\"1.67.0\"}",
            "192.168.1.108",
        )
        .unwrap();

        assert_eq!(bridge.id, "001788FFFE123456");
        assert_eq!(bridge.ip, "192.168.1.108");
    }
}
