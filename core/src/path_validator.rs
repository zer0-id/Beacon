/// 4096 bytes including null terminator for both OSes.
pub const MAX_PATH_BYTES: usize = 4095;
/// 255 Bytes UTF-8 across both systems.
pub const MAX_NAME_BYTES: usize = 255;

/// Characters forbidden in file/folder names (FAT-strict set).
/// Source: AOSP `FileUtils.isValidFatFilenameChar()`
/// https://cs.android.com/android/platform/superproject/main/+/03b7b057c4e76cafc1f96d349adcd60ac0320a5e:frameworks/base/core/java/android/os/FileUtils.java;drc=4ac7fb181a44e5bf484339db21109f76148ebe84;l=1100
/// Last checked: 06-11-2026 ; Commit: 03b7b05
/// Control characters (0x00..=0x1F) are also forbidden
pub const FORBIDDEN_CHARS: [char; 10] = ['"', '*', '/', ':', '<', '>', '?', '\\', '|', '\u{7f}'];

#[derive(Debug, PartialEq, Eq)]
pub enum ValidatorError {
    /// If path is empty or contains many '/'.
    Empty,
    /// Path traversal rejection.
    Dot,
    /// Path traversal rejection.
    DotDot,
    /// Directory/File name in path exceeds MAX_NAME_BYTES.
    NameTooLong { bytes: usize },
    /// Path size exceeds MAX_PATH_BYTES.
    PathTooLong { bytes: usize },
    /// Path contains an invalid char, check FORBIDDEN_CHARS.
    InvalidChar(char),
}

fn is_forbidden_char(c: char) -> bool {
    c <= '\u{1f}' || FORBIDDEN_CHARS.contains(&c)
}

fn validate_component(name: &str) -> Result<(), ValidatorError> {
    if name.is_empty() {
        return Err(ValidatorError::Empty);
    }
    if name.len() > MAX_NAME_BYTES {
        return Err(ValidatorError::NameTooLong { bytes: name.len() });
    }
    if name == "." {
        return Err(ValidatorError::Dot);
    }
    if name == ".." {
        return Err(ValidatorError::DotDot);
    }
    for c in name.chars() {
        if is_forbidden_char(c) {
            return Err(ValidatorError::InvalidChar(c));
        }
    }
    Ok(())
}

/// Validates a sender-supplied path ('/' separated string,
/// not an OS path). Rejects empty components, "." / "..", forbidden chars,
/// and names over MAX_NAME_BYTES.
///
/// Leading, trailing and doubled slashes yield an empty component, so they
/// are rejected as `Empty`.
///
/// NOTE: the MAX_PATH_BYTES check applies to the sender's path only. The caller
/// must re-check the length of the final joined path (download dir + this path).
pub fn validate_path(path: &str) -> Result<(), ValidatorError> {
    if path.len() > MAX_PATH_BYTES {
        return Err(ValidatorError::PathTooLong { bytes: path.len() });
    }
    for component in path.split('/') {
        validate_component(component)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_paths() {
        assert_eq!(validate_path("img.jpg"), Ok(()));
        assert_eq!(validate_path("photos/2026/img.jpg"), Ok(()));
        assert_eq!(validate_path("my file (1).txt"), Ok(()));
    }

    #[test]
    fn rejects_empty_components() {
        assert_eq!(validate_path(""), Err(ValidatorError::Empty));
        assert_eq!(validate_path("a//b"), Err(ValidatorError::Empty));
        assert_eq!(validate_path("/etc/passwd"), Err(ValidatorError::Empty));
        assert_eq!(validate_path("a/b/"), Err(ValidatorError::Empty));
    }

    #[test]
    fn rejects_traversal() {
        assert_eq!(validate_path("../x"), Err(ValidatorError::DotDot));
        assert_eq!(validate_path("a/../b"), Err(ValidatorError::DotDot));
        assert_eq!(validate_path("./a"), Err(ValidatorError::Dot));
    }

    #[test]
    fn rejects_each_forbidden_char() {
        for c in FORBIDDEN_CHARS.iter().filter(|&&c| c != '/') {
            let name = format!("a{c}b");
            assert_eq!(validate_path(&name), Err(ValidatorError::InvalidChar(*c)));
        }
        assert_eq!(
            validate_path("a\tb"),
            Err(ValidatorError::InvalidChar('\t'))
        );
    }

    #[test]
    fn name_length_is_in_bytes() {
        assert_eq!(validate_path(&"a".repeat(255)), Ok(()));
        assert!(matches!(
            validate_path(&"a".repeat(256)),
            Err(ValidatorError::NameTooLong { bytes: 256 })
        ));
        // 'é' is 2 bytes, so 128 of them is 256 bytes even though it's only 128 chars
        assert!(matches!(
            validate_path(&"é".repeat(128)),
            Err(ValidatorError::NameTooLong { .. })
        ));
    }

    #[test]
    fn path_length_boundary() {
        let ok = format!("{}a", "a/".repeat(2047));
        assert_eq!(validate_path(&ok), Ok(()));
        let too_long = format!("{ok}b"); // 4096 bytes
        assert!(matches!(
            validate_path(&too_long),
            Err(ValidatorError::PathTooLong { bytes: 4096 })
        ));
    }
}
