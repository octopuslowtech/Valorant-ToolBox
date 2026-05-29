use std::os::windows::process::CommandExt;
use std::process::Command;

use winreg::enums::*;
use winreg::RegKey;

const CREATE_NO_WINDOW: u32 = 0x08000000;

fn run_cmd(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn set_dword(hive: isize, path: &str, name: &str, value: u32) -> bool {
    let root = RegKey::predef(hive);
    if let Ok((key, _)) = root.create_subkey(path) {
        return key.set_value(name, &value).is_ok();
    }
    false
}

pub fn set_valorant_priority() -> (bool, String) {
    let script = r#"
$p = Get-Process -Name 'VALORANT-Win64-Shipping' -ErrorAction SilentlyContinue;
if ($p) {
    $p.PriorityClass = 'High';
    $p.ProcessorAffinity = [IntPtr]::new((1 -shl [Environment]::ProcessorCount) - 1);
}
$r = Get-Process -Name 'RiotClientServices' -ErrorAction SilentlyContinue;
if ($r) { $r.PriorityClass = 'AboveNormal' }
"#;
    let ok = run_cmd("powershell", &["-Command", script]);
    if ok {
        (true, "Valorant priority set to High".into())
    } else {
        (false, "Valorant not running or access denied".into())
    }
}

pub fn set_high_performance_power() -> (bool, String) {
    let high_perf = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";
    if run_cmd("powercfg", &["/setactive", high_perf]) {
        return (true, "High Performance power plan activated".into());
    }
    let ultimate = "e9a42b02-d5df-448d-aa00-03f14749eb61";
    if run_cmd("powercfg", &["/setactive", ultimate]) {
        return (true, "Ultimate Performance power plan activated".into());
    }
    (false, "Could not set performance power plan".into())
}

pub fn disable_services() -> (bool, String) {
    let services = [
        "Fax",
        "Spooler",
        "TabletInputService",
        "Themes",
        "WSearch",
        "SysMain",
        "DiagTrack",
        "dmwappushservice",
        "MapsBroker",
        "lfsvc",
        "SharedAccess",
        "lltdsvc",
        "AppVClient",
        "NetTcpPortSharing",
        "RemoteAccess",
        "RemoteRegistry",
        "WbioSrvc",
        "WMPNetworkSvc",
        "WpcMonSvc",
        "SessionEnv",
        "TermService",
        "UmRdpService",
        "RpcLocator",
        "WerSvc",
        "Wecsvc",
        "FontCache",
        "stisvc",
        "wisvc",
        "PcaSvc",
        "CscService",
        "defragsvc",
        "wuauserv",
        "UsoSvc",
        "WaaSMedicSvc",
    ];

    let mut disabled = 0;
    for svc in &services {
        if run_cmd("sc", &["config", svc, "start=", "disabled"]) {
            let _ = run_cmd("sc", &["stop", svc]);
            disabled += 1;
        }
    }
    (
        disabled > 0,
        format!("Disabled {}/{} services", disabled, services.len()),
    )
}

pub fn optimize_network() -> (bool, String) {
    let commands: &[&[&str]] = &[
        &["netsh", "int", "tcp", "set", "global", "autotuninglevel=disabled"],
        &["netsh", "int", "tcp", "set", "global", "rss=enabled"],
        &["netsh", "int", "tcp", "set", "global", "rsc=disabled"],
        &["netsh", "int", "tcp", "set", "heuristics", "disabled"],
        &["netsh", "int", "tcp", "set", "global", "nonsackrttresiliency=disabled"],
        &["netsh", "int", "tcp", "set", "supplemental", "internet", "congestionprovider=ctcp"],
    ];

    let mut ok_count = 0;
    for args in commands {
        if run_cmd(args[0], &args[1..]) {
            ok_count += 1;
        }
    }
    (
        ok_count > 0,
        format!("Network tweaks applied ({}/{})", ok_count, commands.len()),
    )
}

pub fn apply_registry_tweaks() -> (bool, String) {
    let r1 = set_dword(
        HKEY_LOCAL_MACHINE,
        r"SYSTEM\CurrentControlSet\Control\PriorityControl",
        "Win32PrioritySeparation",
        38,
    );
    let r2 = set_dword(
        HKEY_LOCAL_MACHINE,
        r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers",
        "HwSchMode",
        2,
    );
    let r3 = set_dword(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Policies\Microsoft\Windows\DataCollection",
        "AllowTelemetry",
        0,
    );

    let count = [r1, r2, r3].iter().filter(|&&x| x).count();
    (
        count > 0,
        format!("Registry tweaks applied ({}/3)", count),
    )
}
pub fn disable_background_apps() -> (bool, String) {
    let r1 = set_dword(
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications",
        "GlobalUserDisabled",
        1,
    );
    let r2 = set_dword(
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\Serialize",
        "StartupDelayInMSec",
        0,
    );
    let r3 = set_dword(
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager",
        "SystemPaneSuggestionsEnabled",
        0,
    );
    let r4 = set_dword(
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager",
        "SoftLandingEnabled",
        0,
    );
    let count = [r1, r2, r3, r4].iter().filter(|&&x| x).count();
    (
        count > 0,
        format!("Background apps disabled ({}/4)", count),
    )
}

pub fn optimize_network_advanced() -> (bool, String) {
    let commands: &[&[&str]] = &[
        &["netsh", "int", "tcp", "set", "global", "timestamps=disabled"],
        &["netsh", "int", "tcp", "set", "global", "initialRto=2000"],
        &["netsh", "int", "tcp", "set", "global", "maxsynretransmissions=2"],
        &["netsh", "interface", "ipv4", "set", "subinterface", "Ethernet", "mtu=1500", "store=persistent"],
        &["netsh", "interface", "ipv4", "set", "subinterface", "Wi-Fi", "mtu=1500", "store=persistent"],
        &["netsh", "interface", "ipv4", "set", "subinterface", "Local Area Connection", "mtu=1500", "store=persistent"],
    ];

    let mut ok_count = 0;
    for args in commands {
        if run_cmd(args[0], &args[1..]) {
            ok_count += 1;
        }
    }
    (
        ok_count > 0,
        format!("Advanced network tweaks applied ({}/{})", ok_count, commands.len()),
    )
}

pub fn run_all<F: FnMut(String)>(mut log: F) {
    let steps: Vec<(&str, (bool, String))> = vec![
        ("Priority", set_valorant_priority()),
        ("Power", set_high_performance_power()),
        ("Services", disable_services()),
        ("Network", optimize_network()),
        ("Network+", optimize_network_advanced()),
        ("Background", disable_background_apps()),
        ("Registry", apply_registry_tweaks()),
    ];
    for (label, (ok, msg)) in steps {
        log(format!(
            "{} [{}] {}",
            label,
            if ok { "OK" } else { "--" },
            msg
        ));
    }
}
