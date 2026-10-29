//! Antigravity launches `cmd /c` using normal Windows argv quoting. Embedded
//! quotes in a command string become literal backslash-quote sequences there.
//! Prefer a quote-free short path; fall back to an encoded PowerShell launcher
//! on volumes where Windows has not assigned an 8.3 filename.
use super::bridge::FLAG;
use std::path::Path;

#[cfg(windows)]
const POWERSHELL_PREFIX: &str =
    "powershell.exe -NoLogo -NoProfile -NonInteractive -EncodedCommand ";
#[cfg(windows)]
const SCRIPT_MARKER: &str = "# ai-usage-widget --antigravity-statusline-bridge\n";

pub fn bridge_command(exe: &Path) -> String {
    #[cfg(windows)]
    {
        if safe_for_cmd(&exe.to_string_lossy()) {
            return format!("{} {FLAG}", exe.display());
        }
        if let Some(short) = short_path(exe).filter(|path| safe_for_cmd(&path.to_string_lossy())) {
            return format!("{} {FLAG}", short.display());
        }
        powershell_command(exe, FLAG)
    }
    #[cfg(not(windows))]
    {
        format!("'{}' {FLAG}", exe.to_string_lossy().replace('\'', "'\\''"))
    }
}

pub fn is_bridge_command(command: &str) -> bool {
    if command.contains(FLAG) {
        return true;
    }
    #[cfg(windows)]
    {
        let bytes = utf16_bytes(SCRIPT_MARKER);
        // Complete base64 groups remain a prefix when the rest of the script
        // is appended. This marker identifies our encoded launcher as well.
        let marker = base64(&bytes[..bytes.len() / 3 * 3]);
        command
            .strip_prefix(POWERSHELL_PREFIX)
            .is_some_and(|script| script.starts_with(&marker))
    }
    #[cfg(not(windows))]
    false
}

#[cfg(windows)]
fn safe_for_cmd(path: &str) -> bool {
    !path
        .chars()
        .any(|c| c.is_whitespace() || "\"&|<>()^%!".contains(c))
}

#[cfg(windows)]
fn short_path(path: &Path) -> Option<std::path::PathBuf> {
    use std::{
        ffi::OsString,
        os::windows::ffi::{OsStrExt, OsStringExt},
    };
    #[link(name = "kernel32")]
    extern "system" {
        fn GetShortPathNameW(long: *const u16, short: *mut u16, length: u32) -> u32;
    }
    let long: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let length = unsafe { GetShortPathNameW(long.as_ptr(), std::ptr::null_mut(), 0) };
    if length == 0 {
        return None;
    }
    let mut short = vec![0u16; length as usize];
    let written = unsafe { GetShortPathNameW(long.as_ptr(), short.as_mut_ptr(), length) };
    if written == 0 || written >= length {
        return None;
    }
    Some(OsString::from_wide(&short[..written as usize]).into())
}

#[cfg(windows)]
fn powershell_command(exe: &Path, arguments: &str) -> String {
    let path = exe.to_string_lossy().replace('\'', "''");
    let arguments = arguments.replace('\'', "''");
    // Forward raw bytes so non-ASCII JSON survives Windows PowerShell's
    // default text encodings. No script files or execution-policy changes.
    let script = format!("{SCRIPT_MARKER}$ErrorActionPreference='Stop'; \
        $p=New-Object System.Diagnostics.Process; \
        $p.StartInfo.FileName='{path}'; $p.StartInfo.Arguments='{arguments}'; \
        $p.StartInfo.UseShellExecute=$false; $p.StartInfo.CreateNoWindow=$true; \
        $p.StartInfo.RedirectStandardInput=$true; $p.StartInfo.RedirectStandardOutput=$true; \
        [void]$p.Start(); \
        [Console]::OpenStandardInput().CopyTo($p.StandardInput.BaseStream); $p.StandardInput.Close(); \
        $p.StandardOutput.BaseStream.CopyTo([Console]::OpenStandardOutput()); \
        $p.WaitForExit(); exit $p.ExitCode");
    format!("{POWERSHELL_PREFIX}{}", base64(&utf16_bytes(&script)))
}

#[cfg(windows)]
fn utf16_bytes(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

#[cfg(windows)]
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::new();
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        encoded.push(ALPHABET[((n >> 18) & 63) as usize] as char);
        encoded.push(ALPHABET[((n >> 12) & 63) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            ALPHABET[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            ALPHABET[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    encoded
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::{
        fs,
        io::Write,
        os::windows::process::CommandExt,
        path::PathBuf,
        process::{Command, Stdio},
    };

    struct Fixture(PathBuf);
    impl Fixture {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("widget agy shell {} {name}", std::process::id()));
            fs::create_dir_all(&dir).unwrap();
            let target = dir.join("native echo.exe");
            fs::copy(
                PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32/findstr.exe"),
                &target,
            )
            .unwrap();
            Self(target)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
            let _ = fs::remove_dir(self.0.parent().unwrap());
        }
    }
    fn run(command: &str) -> std::process::Output {
        let mut child = Command::new("cmd.exe")
            .args(["/c", command])
            .creation_flags(0x0800_0000)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all("{\"quota\":{},\"model\":\"é\"}\r\n".as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    }
    #[test]
    fn encoded_launcher_preserves_stdin_and_stdout_for_paths_with_spaces() {
        let fixture = Fixture::new("encoded");
        let command = powershell_command(&fixture.0, "/c:quota");
        assert!(is_bridge_command(&command));
        let output = run(&command);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            "{\"quota\":{},\"model\":\"é\"}"
        );
    }
    #[test]
    fn short_path_runs_with_antigravity_shell_quoting_and_filters_metacharacters() {
        let fixture = Fixture::new("short");
        if let Some(short) = short_path(&fixture.0).filter(|p| safe_for_cmd(&p.to_string_lossy())) {
            let output = run(&format!("{} /c:quota", short.display()));
            assert!(output.status.success());
            assert!(String::from_utf8_lossy(&output.stdout).contains("quota"));
        }
        for path in [
            "C:/Program Files/app.exe",
            "C:/a&b/app.exe",
            "C:/%USERPROFILE%/app.exe",
        ] {
            assert!(!safe_for_cmd(path));
        }
        assert!(safe_for_cmd("C:/PROGRA~1/app.exe"));
        assert!(!is_bridge_command(
            "powershell.exe -NoProfile -Command unrelated"
        ));
    }
}
