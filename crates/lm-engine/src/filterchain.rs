//! Loading `libpipewire-module-filter-chain` into our own context, and
//! building the SPA-JSON module args for strip/bus DSP graphs.
//!
//! Modules are loaded in-process so every virtual device dies with the app —
//! `pw-dump` must show zero `lm.*` nodes after exit.

use std::ffi::CString;
use std::ptr::NonNull;

use pipewire::context::ContextRc;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ModuleError {
    #[error("module args contained an interior NUL byte")]
    BadArgs(#[from] std::ffi::NulError),
    #[error("pw_context_load_module failed: {0}")]
    LoadFailed(std::io::Error),
}

/// An in-process PipeWire module. Destroying this unloads the module and
/// removes every node it created from the graph.
///
/// Must be dropped before the [`ContextRc`] it was loaded into; holding a
/// clone of the context here guarantees that ordering.
pub struct LoadedModule {
    ptr: NonNull<pipewire::sys::pw_impl_module>,
    _context: ContextRc,
}

impl LoadedModule {
    pub fn load(context: &ContextRc, name: &str, args: &str) -> Result<Self, ModuleError> {
        let cname = CString::new(name)?;
        let cargs = CString::new(args)?;
        // SAFETY: context pointer is valid for the lifetime of ContextRc, and
        // we keep a ContextRc clone alive for as long as the module exists.
        let ptr = unsafe {
            pipewire::sys::pw_context_load_module(
                context.as_raw_ptr(),
                cname.as_ptr(),
                cargs.as_ptr(),
                std::ptr::null_mut(),
            )
        };
        match NonNull::new(ptr) {
            Some(ptr) => Ok(Self { ptr, _context: context.clone() }),
            None => Err(ModuleError::LoadFailed(std::io::Error::last_os_error())),
        }
    }
}

impl Drop for LoadedModule {
    fn drop(&mut self) {
        // SAFETY: ptr came from pw_context_load_module and the context is
        // still alive (we hold an Rc on it).
        unsafe { pipewire::sys::pw_impl_module_destroy(self.ptr.as_ptr()) };
    }
}

const LV2_GATE: &str = "http://lsp-plug.in/plugins/lv2/gate_stereo";
const LV2_COMP: &str = "http://lsp-plug.in/plugins/lv2/compressor_stereo";
const LV2_EQ: &str = "http://lsp-plug.in/plugins/lv2/para_equalizer_x8_stereo";
const LV2_LIMITER: &str = "http://lsp-plug.in/plugins/lv2/limiter_stereo";

/// Every input strip's DSP: gate → comp → EQ → stereo image.
///
/// The stereo image (mono / reverse / balance) is a 2x2 gain matrix built from
/// two builtin mixers: `panl` produces the left output from (L, R) and `panr`
/// the right. Their `Gain 1` / `Gain 2` controls are the matrix rows from
/// [`lm_protocol::PanParams::matrix`]; the defaults here are the identity, so
/// a strip passes audio unchanged until the engine says otherwise.
pub fn strip_graph() -> serde_json::Value {
    json!({
        "nodes": [
            { "type": "lv2", "name": "gate", "plugin": LV2_GATE },
            { "type": "lv2", "name": "comp", "plugin": LV2_COMP },
            { "type": "lv2", "name": "eq",   "plugin": LV2_EQ },
            { "type": "builtin", "name": "panl", "label": "mixer", "control": { "Gain 1": 1.0, "Gain 2": 0.0 } },
            { "type": "builtin", "name": "panr", "label": "mixer", "control": { "Gain 1": 0.0, "Gain 2": 1.0 } },
        ],
        "links": [
            { "output": "gate:out_l", "input": "comp:in_l" },
            { "output": "gate:out_r", "input": "comp:in_r" },
            { "output": "comp:out_l", "input": "eq:in_l" },
            { "output": "comp:out_r", "input": "eq:in_r" },
            // Each EQ output feeds both mixers (an output may fan out).
            { "output": "eq:out_l", "input": "panl:In 1" },
            { "output": "eq:out_r", "input": "panl:In 2" },
            { "output": "eq:out_l", "input": "panr:In 1" },
            { "output": "eq:out_r", "input": "panr:In 2" },
        ],
        "inputs":  [ "gate:in_l", "gate:in_r" ],
        "outputs": [ "panl:Out", "panr:Out" ],
    })
}

/// Module args for a virtual input strip: an `Audio/Sink` other apps play
/// into, processed through gate → comp → EQ.
///
/// `sink_name` becomes the visible device (e.g. `lm.strip.v1`); the processed
/// output appears as stream node `<sink_name>.out`.
///
/// When `autoconnect_output` is true the output stream connects to the default
/// sink (useful for testing a strip in isolation before buses exist). In the
/// real graph it stays false and the routing matrix links it to bus inputs.
pub fn virtual_strip_args(sink_name: &str, label: &str, autoconnect_output: bool) -> String {
    let mut playback_props = json!({
        "node.name": format!("{sink_name}.out"),
        "node.description": format!("{label} (out)"),
        "media.class": "Stream/Output/Audio",
        "node.virtual": true,
            "state.restore-props": false,
        "node.dont-reconnect": true,
    });
    if !autoconnect_output {
        playback_props["node.autoconnect"] = json!(false);
    }
    json!({
        "node.description": label,
        "media.name": label,
        "filter.graph": strip_graph(),
        "capture.props": {
            "node.name": sink_name,
            "node.description": label,
            "media.class": "Audio/Sink",
            "audio.position": [ "FL", "FR" ],
            "node.virtual": true,
            "state.restore-props": false,
        },
        "playback.props": playback_props,
    })
    .to_string()
}

/// Module args for a hardware input strip: captures from an existing source
/// node (`hw_target` = its `node.name`), processed through the same
/// gate → comp → EQ graph. Output appears as `<strip_name>.out`.
pub fn hardware_strip_args(strip_name: &str, label: &str, hw_target: &str) -> String {
    json!({
        "node.description": label,
        "media.name": label,
        "filter.graph": strip_graph(),
        "capture.props": {
            "node.name": format!("{strip_name}.cap"),
            "node.description": format!("{label} (capture)"),
            "target.object": hw_target,
            "stream.capture.sink": false,
            "node.dont-reconnect": true,
            "node.virtual": true,
            "state.restore-props": false,
        },
        "playback.props": {
            "node.name": format!("{strip_name}.out"),
            "node.description": format!("{label} (out)"),
            "media.class": "Stream/Output/Audio",
            "node.autoconnect": false,
            "node.dont-reconnect": true,
            "node.virtual": true,
            "state.restore-props": false,
        },
    })
    .to_string()
}

/// Module args for a VBAN input strip: the same gate → comp → EQ graph, but
/// its input is a plain non-autoconnecting stream rather than a sink, so apps
/// never see it as a playback device. The VBAN receiver stream
/// (`<strip_name>.vban`) is linked into it by the routing reconciler.
pub fn vban_strip_args(strip_name: &str, label: &str) -> String {
    json!({
        "node.description": label,
        "media.name": label,
        "filter.graph": strip_graph(),
        "capture.props": {
            "node.name": strip_name,
            "node.description": format!("{label} (VBAN in)"),
            "media.class": "Stream/Input/Audio",
            "audio.position": [ "FL", "FR" ],
            "node.autoconnect": false,
            "node.dont-reconnect": true,
            "node.virtual": true,
            "state.restore-props": false,
        },
        "playback.props": {
            "node.name": format!("{strip_name}.out"),
            "node.description": format!("{label} (out)"),
            "media.class": "Stream/Output/Audio",
            "node.autoconnect": false,
            "node.dont-reconnect": true,
            "node.virtual": true,
            "state.restore-props": false,
        },
    })
    .to_string()
}

/// Props shared by every VBAN stream we create: invisible to the session
/// manager's linking policy, linked only by our own reconciler.
fn vban_stream_props(node_name: &str, description: &str, media_class: &str) -> serde_json::Value {
    json!({
        "node.name": node_name,
        "node.description": description,
        "media.class": media_class,
        "node.autoconnect": false,
        "node.dont-reconnect": true,
        "node.virtual": true,
        "state.restore-props": false,
    })
}

/// Module args for `libpipewire-module-vban-send`: a capture stream
/// (`node_name`) whose input is packetized and sent to `cfg.host:cfg.port`.
/// `cfg` must already be [`lm_protocol::VbanSend::validated`].
pub fn vban_send_args(node_name: &str, label: &str, cfg: &lm_protocol::VbanSend) -> String {
    json!({
        "destination.ip": cfg.host,
        "destination.port": cfg.port,
        "sess.name": cfg.stream_name,
        "sess.media": "audio",
        "audio.format": "S16LE",
        "audio.rate": cfg.rate,
        "audio.channels": 2,
        "audio.position": [ "FL", "FR" ],
        "stream.props": vban_stream_props(node_name, &format!("{label} (VBAN out)"), "Stream/Input/Audio"),
    })
    .to_string()
}

/// One stream a VBAN receiver should accept, and the node it becomes.
#[derive(Debug, Clone)]
pub struct VbanRecvStream {
    pub node_name: String,
    pub label: String,
    pub stream_name: String,
    /// Empty = accept any sender.
    pub sender_ip: String,
    pub latency_ms: u32,
}

/// Module args for `libpipewire-module-vban-recv` listening on `port` on all
/// interfaces. Only one socket can own a UDP port, so every stream expected on
/// that port goes through one module:
///
/// - PipeWire ≥ 1.4 creates a stream per incoming VBAN session and consults
///   `stream.rules`; each rule matches one strip's stream name (and sender, if
///   set) and names the node after that strip. Sessions nobody asked for are
///   ignored.
/// - Older PipeWire has no rules and creates a single stream up front from
///   `stream.props`, which therefore carries the *first* strip's identity.
///
/// Panics if `streams` is empty (callers unload the module instead).
pub fn vban_recv_args(port: u16, streams: &[VbanRecvStream]) -> String {
    let first = streams.first().expect("vban_recv_args needs at least one stream");
    let rules: Vec<serde_json::Value> = streams
        .iter()
        .map(|s| {
            let mut m = json!({ "sess.name": s.stream_name });
            if !s.sender_ip.is_empty() {
                m["vban.ip"] = json!(s.sender_ip);
            }
            let mut create = vban_stream_props(&s.node_name, &format!("{} (VBAN)", s.label), "Stream/Output/Audio");
            create["sess.latency.msec"] = json!(s.latency_ms);
            json!({ "matches": [ m ], "actions": { "create-stream": create } })
        })
        .collect();
    json!({
        "source.ip": "0.0.0.0",
        "source.port": port,
        "sess.latency.msec": first.latency_ms,
        // Pre-1.4 the stream format is fixed here; 1.4+ takes it from the
        // packet header and ignores these.
        "audio.rate": 48000,
        "audio.channels": 2,
        "audio.position": [ "FL", "FR" ],
        "stream.props": vban_stream_props(&first.node_name, &format!("{} (VBAN)", first.label), "Stream/Output/Audio"),
        "stream.rules": rules,
    })
    .to_string()
}

/// Whether the running libpipewire's vban-recv understands `stream.rules`
/// (per-session streams, name/sender filtering): added in PipeWire 1.4.
pub fn vban_recv_has_rules(version: &str) -> bool {
    let mut it = version.split('.').map(|p| p.trim().parse::<u32>().unwrap_or(0));
    let (major, minor) = (it.next().unwrap_or(0), it.next().unwrap_or(0));
    (major, minor) >= (1, 4)
}

/// Module args for an output bus.
///
/// `hw_target` = Some(node.name of a hardware sink) makes an A-bus whose
/// output plays to that device; None makes a B-bus that appears as a
/// microphone (`media.class = Audio/Source`) for Discord/OBS/etc.
pub fn bus_args(bus_name: &str, label: &str, hw_target: Option<&str>) -> String {
    let playback_props = match hw_target {
        Some(target) => json!({
            "node.name": format!("{bus_name}.out"),
            "node.description": format!("{label} (out)"),
            "target.object": target,
            "node.dont-reconnect": true,
            "node.virtual": true,
            "state.restore-props": false,
        }),
        None => json!({
            "node.name": bus_name,
            "node.description": label,
            "media.class": "Audio/Source",
            "audio.position": [ "FL", "FR" ],
            "node.virtual": true,
            "state.restore-props": false,
        }),
    };
    json!({
        "node.description": label,
        "media.name": label,
        // EQ then limiter, so EQ boosts can never push the bus past it.
        "filter.graph": {
            "nodes": [
                { "type": "lv2", "name": "eq",  "plugin": LV2_EQ },
                { "type": "lv2", "name": "lim", "plugin": LV2_LIMITER },
            ],
            "links": [
                { "output": "eq:out_l", "input": "lim:in_l" },
                { "output": "eq:out_r", "input": "lim:in_r" },
            ],
            "inputs":  [ "eq:in_l", "eq:in_r" ],
            "outputs": [ "lim:out_l", "lim:out_r" ],
        },
        "capture.props": {
            "node.name": format!("{bus_name}.in"),
            "node.description": format!("{label} (in)"),
            "media.class": "Stream/Input/Audio",
            "node.autoconnect": false,
            "node.dont-reconnect": true,
            "node.virtual": true,
            "state.restore-props": false,
        },
        "playback.props": playback_props,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn parse(args: &str) -> Value {
        serde_json::from_str(args).expect("module args must be valid JSON")
    }

    fn every_props_block(v: &Value) -> Vec<&Value> {
        ["capture.props", "playback.props"].iter().filter_map(|k| v.get(k)).collect()
    }

    fn all_args() -> Vec<(&'static str, String)> {
        vec![
            ("virtual strip", virtual_strip_args("lm.strip.1", "System", false)),
            ("virtual strip autoconnected", virtual_strip_args("lm.strip.1", "System", true)),
            ("hardware strip", hardware_strip_args("lm.strip.2", "Mic", "alsa_input.usb-foo")),
            ("A bus", bus_args("lm.bus.a1", "Speakers", Some("alsa_output.usb-foo"))),
            ("B bus", bus_args("lm.bus.b1", "Stream Mic", None)),
            ("VBAN strip", vban_strip_args("lm.strip.6", "Remote")),
        ]
    }

    fn recv_stream(id: u32, name: &str, sender: &str) -> VbanRecvStream {
        VbanRecvStream {
            node_name: format!("lm.strip.{id}.vban"),
            label: format!("Remote {id}"),
            stream_name: name.into(),
            sender_ip: sender.into(),
            latency_ms: 80,
        }
    }

    /// The VBAN modules take a flat `stream.props` instead of
    /// capture/playback blocks, but the same invariants hold.
    fn vban_stream_blocks() -> Vec<(String, Value)> {
        let send = parse(&vban_send_args("lm.bus.a1.vban", "Speakers", &lm_protocol::VbanSend::default()));
        let recv = parse(&vban_recv_args(6980, &[recv_stream(6, "Stream1", ""), recv_stream(7, "Game", "10.0.0.2")]));
        let mut out = vec![
            ("send".to_string(), send["stream.props"].clone()),
            ("recv".to_string(), recv["stream.props"].clone()),
        ];
        for (i, r) in recv["stream.rules"].as_array().unwrap().iter().enumerate() {
            out.push((format!("recv rule {i}"), r["actions"]["create-stream"].clone()));
        }
        out
    }

    #[test]
    fn vban_streams_are_ours_to_link() {
        for (what, props) in vban_stream_blocks() {
            assert_eq!(props["state.restore-props"], Value::Bool(false), "{what}");
            assert_eq!(props["node.virtual"], Value::Bool(true), "{what}");
            assert_eq!(
                props["node.autoconnect"],
                Value::Bool(false),
                "{what}: the session manager must not wire VBAN streams to the default device"
            );
        }
    }

    #[test]
    fn vban_strip_input_is_not_a_playback_device() {
        let v = parse(&vban_strip_args("lm.strip.6", "Remote"));
        assert_eq!(v["capture.props"]["node.name"], "lm.strip.6", "control_node() of a VBAN strip");
        assert_eq!(v["capture.props"]["media.class"], "Stream/Input/Audio");
        assert_eq!(v["capture.props"]["node.autoconnect"], Value::Bool(false));
        assert_eq!(v["playback.props"]["node.name"], "lm.strip.6.out");
    }

    #[test]
    fn vban_send_targets_the_configured_host() {
        let cfg = lm_protocol::VbanSend {
            enabled: true,
            stream_name: "Mix".into(),
            host: "192.168.1.50".into(),
            port: 6981,
            rate: 44100,
        };
        let v = parse(&vban_send_args("lm.bus.b1.vban", "Stream Mic", &cfg));
        assert_eq!(v["destination.ip"], "192.168.1.50");
        assert_eq!(v["destination.port"], 6981);
        assert_eq!(v["sess.name"], "Mix");
        assert_eq!(v["audio.rate"], 44100);
        assert_eq!(v["stream.props"]["node.name"], "lm.bus.b1.vban");
        assert_eq!(v["stream.props"]["media.class"], "Stream/Input/Audio");
    }

    /// One rule per strip, matching on stream name and (optionally) sender,
    /// naming the created node after the strip so the reconciler can link it.
    #[test]
    fn vban_recv_rules_route_each_session_to_its_strip() {
        let v = parse(&vban_recv_args(6980, &[recv_stream(6, "Stream1", ""), recv_stream(7, "Game", "10.0.0.2")]));
        assert_eq!(v["source.ip"], "0.0.0.0", "the module default (127.0.0.1) would only hear this machine");
        assert_eq!(v["source.port"], 6980);
        let rules = v["stream.rules"].as_array().unwrap();
        assert_eq!(rules.len(), 2);

        assert_eq!(rules[0]["matches"][0]["sess.name"], "Stream1");
        assert!(rules[0]["matches"][0].get("vban.ip").is_none(), "no sender filter = any sender");
        assert_eq!(rules[0]["actions"]["create-stream"]["node.name"], "lm.strip.6.vban");
        assert_eq!(rules[0]["actions"]["create-stream"]["sess.latency.msec"], 80);

        assert_eq!(rules[1]["matches"][0]["sess.name"], "Game");
        assert_eq!(rules[1]["matches"][0]["vban.ip"], "10.0.0.2");
        assert_eq!(rules[1]["actions"]["create-stream"]["node.name"], "lm.strip.7.vban");

        // Pre-1.4 fallback: the single up-front stream is the first strip's.
        assert_eq!(v["stream.props"]["node.name"], "lm.strip.6.vban");
    }

    #[test]
    fn vban_rules_need_pipewire_1_4() {
        assert!(!vban_recv_has_rules("1.0.5"));
        assert!(!vban_recv_has_rules("1.2.7"));
        assert!(vban_recv_has_rules("1.4.0"));
        assert!(vban_recv_has_rules("1.5.81"));
        assert!(vban_recv_has_rules("2.0.0"));
        assert!(!vban_recv_has_rules("garbage"));
    }

    /// WirePlumber's restore-stream overwrites volume/mute on node appearance.
    /// Every node we create must opt out, or the mixer fights the session
    /// manager on every device change.
    #[test]
    fn every_node_opts_out_of_wireplumber_restore() {
        for (what, args) in all_args() {
            let v = parse(&args);
            let blocks = every_props_block(&v);
            assert_eq!(blocks.len(), 2, "{what}: expected capture + playback props");
            for props in blocks {
                assert_eq!(
                    props.get("state.restore-props"),
                    Some(&Value::Bool(false)),
                    "{what}: node {:?} would let WirePlumber restore our volume",
                    props.get("node.name")
                );
            }
        }
    }

    #[test]
    fn every_node_is_marked_virtual() {
        for (what, args) in all_args() {
            let v = parse(&args);
            for props in every_props_block(&v) {
                assert_eq!(props.get("node.virtual"), Some(&Value::Bool(true)), "{what}");
            }
        }
    }

    /// Node names are the identity the profile, routing matrix, and meter taps
    /// all key on — they must match the helpers in lm-protocol exactly.
    #[test]
    fn virtual_strip_names_sink_and_output() {
        let v = parse(&virtual_strip_args("lm.strip.3", "Music", false));
        assert_eq!(v["capture.props"]["node.name"], "lm.strip.3");
        assert_eq!(v["capture.props"]["media.class"], "Audio/Sink", "apps must be able to play into it");
        assert_eq!(v["playback.props"]["node.name"], "lm.strip.3.out");
        assert_eq!(v["playback.props"]["media.class"], "Stream/Output/Audio");
    }

    #[test]
    fn hardware_strip_captures_from_its_target() {
        let v = parse(&hardware_strip_args("lm.strip.1", "Mic", "alsa_input.usb-zoom"));
        assert_eq!(v["capture.props"]["node.name"], "lm.strip.1.cap");
        assert_eq!(v["capture.props"]["target.object"], "alsa_input.usb-zoom");
        assert_eq!(v["capture.props"]["stream.capture.sink"], Value::Bool(false), "capture the source, not its monitor");
        assert_eq!(v["playback.props"]["node.name"], "lm.strip.1.out");
    }

    /// The routing matrix owns every link; letting the session manager
    /// autoconnect strip outputs would duplicate audio paths.
    #[test]
    fn strip_outputs_do_not_autoconnect() {
        for args in [
            virtual_strip_args("lm.strip.1", "System", false),
            hardware_strip_args("lm.strip.2", "Mic", "alsa_input.usb-foo"),
        ] {
            let v = parse(&args);
            assert_eq!(v["playback.props"]["node.autoconnect"], Value::Bool(false));
        }
    }

    /// The isolation escape hatch used by the spike example.
    #[test]
    fn autoconnect_flag_opens_the_output() {
        let v = parse(&virtual_strip_args("lm.strip.1", "System", true));
        assert!(v["playback.props"].get("node.autoconnect").is_none());
    }

    #[test]
    fn a_bus_plays_to_hardware() {
        let v = parse(&bus_args("lm.bus.a1", "Speakers", Some("alsa_output.hdmi")));
        assert_eq!(v["capture.props"]["node.name"], "lm.bus.a1.in");
        assert_eq!(v["playback.props"]["node.name"], "lm.bus.a1.out");
        assert_eq!(v["playback.props"]["target.object"], "alsa_output.hdmi");
    }

    /// B buses are the virtual microphones Discord/OBS select; the node must
    /// carry the bus name itself, since that is what `tap_node` meters.
    #[test]
    fn b_bus_is_a_virtual_microphone() {
        let v = parse(&bus_args("lm.bus.b1", "Stream Mic", None));
        assert_eq!(v["playback.props"]["node.name"], "lm.bus.b1");
        assert_eq!(v["playback.props"]["media.class"], "Audio/Source");
        assert!(v["playback.props"].get("target.object").is_none());
    }

    #[test]
    fn strip_dsp_chain_is_gate_comp_eq_then_stereo_image() {
        let v = parse(&virtual_strip_args("lm.strip.1", "System", false));
        let names: Vec<&str> = v["filter.graph"]["nodes"]
            .as_array()
            .expect("nodes array")
            .iter()
            .map(|n| n["name"].as_str().expect("node name"))
            .collect();
        assert_eq!(names, ["gate", "comp", "eq", "panl", "panr"]);
        // The Props keys the engine writes are "<node>:<port>"; these prefixes
        // are the contract between filterchain.rs and engine.rs.
        assert_eq!(v["filter.graph"]["inputs"][0], "gate:in_l");
        assert_eq!(v["filter.graph"]["outputs"], json!(["panl:Out", "panr:Out"]));
    }

    /// All three strip kinds share one graph, so a DSP stage can't go missing
    /// from one of them.
    #[test]
    fn every_strip_kind_uses_the_same_graph() {
        let g = strip_graph();
        assert_eq!(parse(&virtual_strip_args("s", "S", false))["filter.graph"], g);
        assert_eq!(parse(&hardware_strip_args("s", "S", "alsa_input.x"))["filter.graph"], g);
        assert_eq!(parse(&vban_strip_args("s", "S"))["filter.graph"], g);
    }

    /// The stereo-image mixers start as the identity: a fresh strip is
    /// unchanged until the engine pushes its pan settings.
    #[test]
    fn stereo_image_mixers_default_to_pass_through() {
        let g = strip_graph();
        let node = |n: &str| g["nodes"].as_array().unwrap().iter().find(|x| x["name"] == n).unwrap().clone();
        assert_eq!(node("panl")["label"], "mixer");
        assert_eq!(node("panl")["control"], json!({ "Gain 1": 1.0, "Gain 2": 0.0 }));
        assert_eq!(node("panr")["control"], json!({ "Gain 1": 0.0, "Gain 2": 1.0 }));
        // Both EQ outputs reach both mixers, in L, R order.
        let links = g["links"].as_array().unwrap();
        for (out, inp) in [("eq:out_l", "panl:In 1"), ("eq:out_r", "panl:In 2"), ("eq:out_l", "panr:In 1"), ("eq:out_r", "panr:In 2")] {
            assert!(links.contains(&json!({ "output": out, "input": inp })), "missing {out} -> {inp}");
        }
    }

    #[test]
    fn bus_dsp_is_eq_then_limiter() {
        let v = parse(&bus_args("lm.bus.a1", "Speakers", None));
        let g = &v["filter.graph"];
        assert_eq!(g["nodes"][0]["name"], "eq");
        assert_eq!(g["nodes"][0]["plugin"], LV2_EQ);
        assert_eq!(g["nodes"][1]["name"], "lim");
        assert_eq!(g["inputs"], json!(["eq:in_l", "eq:in_r"]));
        assert_eq!(g["outputs"], json!(["lim:out_l", "lim:out_r"]), "the limiter is always last");
        assert_eq!(g["links"], json!([
            { "output": "eq:out_l", "input": "lim:in_l" },
            { "output": "eq:out_r", "input": "lim:in_r" },
        ]));
    }

    #[test]
    fn module_args_reject_interior_nul() {
        // LoadedModule::load builds a CString; a NUL must surface as BadArgs
        // rather than a panic. (Labels come from user-editable profiles.)
        let err = CString::new("has\0nul").unwrap_err();
        let module_err: ModuleError = err.into();
        assert!(matches!(module_err, ModuleError::BadArgs(_)));
    }
}
