//! Protocol versioning for client-server compatibility

/// Current protocol version
/// Increment this when making breaking changes to the protocol
pub const PROTOCOL_VERSION: u32 = 1;

/// Check if a client and server are compatible
///
/// For now: exact match required
/// Later: can add backward compatibility ranges
pub fn is_compatible(client_version: u32, server_version: u32) -> bool {
    client_version == server_version
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compatibility() {
        assert!(is_compatible(1, 1));
        assert!(!is_compatible(1, 2));
        assert!(!is_compatible(2, 1));
    }
}
