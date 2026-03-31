use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsysPathType {
    None,
    SimpleWindows,
    EscapedWindows,
    WindowsPathList,
    Unc,
    EscapedPath,
    Rooted,
    PosixPathList,
    Relative,
    Url,
}

pub fn classify_path(input: &str) -> MsysPathType {
    if input.is_empty() {
        return MsysPathType::None;
    }

    let lower = input.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("file://") {
        return MsysPathType::Url;
    }

    if input.starts_with("//") {
        return MsysPathType::Unc;
    }

    if input.starts_with("/\\") {
        return MsysPathType::EscapedWindows;
    }

    if looks_like_windows_drive(input) {
        return MsysPathType::SimpleWindows;
    }

    if input.contains(';') && input.contains(':') {
        return MsysPathType::WindowsPathList;
    }

    if input.contains(':') && input.contains('/') {
        return MsysPathType::PosixPathList;
    }

    if input.starts_with("/") {
        return MsysPathType::Rooted;
    }

    if input.starts_with('\\') {
        return MsysPathType::EscapedPath;
    }

    MsysPathType::Relative
}

fn looks_like_windows_drive(input: &str) -> bool {
    let bytes = input.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

pub fn posix_to_windows(input: &str, root: &Path) -> String {
    match classify_path(input) {
        MsysPathType::Rooted => {
            let mut out = root.to_string_lossy().to_string();
            if !out.ends_with('\\') {
                out.push('\\');
            }
            out.push_str(input.trim_start_matches('/').replace('/', "\\").as_str());
            out
        }
        MsysPathType::PosixPathList => input
            .split(':')
            .map(|segment| posix_to_windows(segment, root))
            .collect::<Vec<_>>()
            .join(";"),
        MsysPathType::EscapedWindows => input.trim_start_matches('/').replace('/', "\\"),
        MsysPathType::EscapedPath => input.replace('/', "\\"),
        _ => input.to_string(),
    }
}

pub fn windows_to_posix(input: &str) -> String {
    match classify_path(input) {
        MsysPathType::SimpleWindows => {
            let drive = &input[0..1];
            let rest = input[2..].replace('\\', "/");
            format!("/{drive}{rest}")
        }
        MsysPathType::WindowsPathList => input
            .split(';')
            .map(windows_to_posix)
            .collect::<Vec<_>>()
            .join(":"),
        MsysPathType::Unc => input.replace('\\', "/"),
        _ => input.replace('\\', "/"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_windows_drive() {
        assert_eq!(classify_path("C:\\Windows"), MsysPathType::SimpleWindows);
    }

    #[test]
    fn converts_windows_path_to_posix() {
        assert_eq!(windows_to_posix("C:\\msys64\\usr\\bin"), "/C/msys64/usr/bin");
    }

    #[test]
    fn converts_posix_rooted_to_windows() {
        let root = Path::new("C:\\msys64");
        assert_eq!(posix_to_windows("/usr/bin", root), "C:\\msys64\\usr\\bin");
    }
}
