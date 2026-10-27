//! Install check for the host machine.
//!
//! The reference desktops are Ubuntu 22.04 and Debian 12. A newer Ubuntu or
//! Debian still counts as a supported OS. Missing camera and display modules
//! are reported and do not fail the check: the host process itself does not
//! need them to pair.

use std::fs;
use std::io::{self, Write};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub os_supported: bool,
    pub lines: Vec<String>,
}

pub fn assess_os_release(text: &str) -> Report {
    let mut values = std::collections::BTreeMap::<String, String>::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        values.insert(key.to_string(), value.trim_matches('"').to_string());
    }
    let id = values.get("ID").map(String::as_str).unwrap_or("unknown");
    let version = values.get("VERSION_ID").map(String::as_str).unwrap_or("0");
    let pretty = values
        .get("PRETTY_NAME")
        .cloned()
        .unwrap_or_else(|| format!("{id} {version}"));
    let supported = match id {
        "ubuntu" => version_at_least(version, "22.04"),
        "debian" => version_at_least(version, "12"),
        _ => false,
    };
    let mut lines = vec![format!("ОС: {pretty}")];
    if supported {
        lines.push(
            "Пол хоста: подходит. Эталон — Ubuntu 22.04 и Debian 12, более новая версия тоже годится для ядра."
                .to_string(),
        );
    } else {
        lines.push(
            "Пол хоста: не подходит. Нужны Ubuntu 22.04+ или Debian 12+. Windows 10 1809 ставится отдельным установщиком."
                .to_string(),
        );
    }
    Report {
        os_supported: supported,
        lines,
    }
}

pub fn run() -> io::Result<i32> {
    let release = fs::read_to_string("/etc/os-release").unwrap_or_default();
    let mut report = assess_os_release(&release);
    if let Ok(uname) = Command::new("uname").arg("-r").output() {
        let kernel = String::from_utf8_lossy(&uname.stdout);
        report.lines.push(format!("Ядро: {}", kernel.trim()));
    }
    report.lines.push(component(
        "usbmuxd",
        command_exists("usbmuxd"),
        "кабель к iPhone и iPad",
    ));
    report.lines.push(component(
        "v4l2loopback",
        module_present("v4l2loopback"),
        "виртуальная камера",
    ));
    report.lines.push(component(
        "evdi",
        module_present("evdi"),
        "дополнительный монитор",
    ));
    let audio = command_exists("pactl") || command_exists("pw-cli");
    report.lines.push(component(
        "PulseAudio или PipeWire",
        audio,
        "виртуальный микрофон",
    ));
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "нет".to_string());
    report.lines.push(format!(
        "Сеанс рабочего стола: {session}. Эталон — GNOME on Xorg и KDE Plasma X11."
    ));
    let stdout = io::stdout();
    let mut out = stdout.lock();
    writeln!(out, "Span doctor")?;
    for line in &report.lines {
        writeln!(out, "{line}")?;
    }
    if report.os_supported {
        writeln!(
            out,
            "Итог: хост на эту машину ставить можно. Модули камеры, звука и монитора ставятся отдельно и без них сопряжение уже работает."
        )?;
        Ok(0)
    } else {
        writeln!(out, "Итог: эту ОС хост не поддерживает.")?;
        Ok(1)
    }
}

fn component(name: &str, present: bool, purpose: &str) -> String {
    if present {
        format!("{name}: есть ({purpose})")
    } else {
        format!("{name}: нет ({purpose})")
    }
}

fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {name}"))
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn module_present(name: &str) -> bool {
    fs::read_to_string("/proc/modules")
        .map(|text| text.lines().any(|line| line.starts_with(name)))
        .unwrap_or(false)
}

fn version_at_least(found: &str, minimum: &str) -> bool {
    fn parts(text: &str) -> (u32, u32) {
        let mut iter = text.split('.');
        let major = iter.next().and_then(|part| part.parse().ok()).unwrap_or(0);
        let minor = iter.next().and_then(|part| part.parse().ok()).unwrap_or(0);
        (major, minor)
    }
    parts(found) >= parts(minimum)
}

#[cfg(test)]
mod tests {
    use super::assess_os_release;

    #[test]
    fn ubuntu_22_and_newer_are_supported() {
        let release = "ID=ubuntu\nVERSION_ID=\"22.04\"\nPRETTY_NAME=\"Ubuntu 22.04 LTS\"\n";
        assert!(assess_os_release(release).os_supported);
        let newer = "ID=ubuntu\nVERSION_ID=\"24.04\"\nPRETTY_NAME=\"Ubuntu 24.04 LTS\"\n";
        assert!(assess_os_release(newer).os_supported);
    }

    #[test]
    fn ubuntu_20_and_debian_11_are_rejected() {
        let ubuntu = "ID=ubuntu\nVERSION_ID=\"20.04\"\n";
        assert!(!assess_os_release(ubuntu).os_supported);
        let debian = "ID=debian\nVERSION_ID=\"11\"\n";
        assert!(!assess_os_release(debian).os_supported);
        let bookworm = "ID=debian\nVERSION_ID=\"12\"\n";
        assert!(assess_os_release(bookworm).os_supported);
    }
}
