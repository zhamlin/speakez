// The mumble version format (v2) is a uint64:
// major   minor   patch   reserved/unused
// 0xFFFF  0xFFFF  0xFFFF  0xFFFF
// (big-endian)
use std::fmt::Display;

#[derive(Clone, Copy, Debug)]
pub struct Version(u64);

impl Version {
    const MASK: u64 = 0xFFFF;
    const MAJOR_OFFSET: u64 = 48;
    const MINOR_OFFSET: u64 = 32;
    const PATCH_OFFSET: u64 = 16;

    pub fn new(major: u16, minor: u16, patch: u16) -> Self {
        let major = (major as u64) << Self::MAJOR_OFFSET;
        let minor = (minor as u64) << Self::MINOR_OFFSET;
        let patch = (patch as u64) << Self::PATCH_OFFSET;
        Version(major | minor | patch)
    }

    pub fn from_u64(input: u64) -> Self {
        Version(input)
    }

    pub fn to_u64(&self) -> u64 {
        self.0
    }

    pub fn major(&self) -> u16 {
        ((self.0 >> Self::MAJOR_OFFSET) & Version::MASK)
            .try_into()
            .unwrap()
    }

    pub fn minor(&self) -> u16 {
        ((self.0 >> Self::MINOR_OFFSET) & Version::MASK)
            .try_into()
            .unwrap()
    }

    pub fn patch(&self) -> u16 {
        ((self.0 >> Self::PATCH_OFFSET) & Version::MASK)
            .try_into()
            .unwrap()
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major(), self.minor(), self.patch())
    }
}
#[cfg(test)]
mod tests {
    use crate::mumble::control::MessageType;

    use super::*;

    #[test]
    fn message_type_u16_conversion() {
        for n in 0..MessageType::SuggestConfig.to_u16() + 1 {
            let m = MessageType::from_u16(n)
                .unwrap_or_else(|| panic!("{n} should map to a message type"));
            assert_eq!(m.to_u16(), n, "{:?} should equal {}", m, n);
        }
    }

    #[test]
    fn version_from_u64() {
        let major = 1;
        let minor = 5;
        let patch = 0;

        let v = Version::from_u64(281496451547136);
        assert_eq!(v.major(), major);
        assert_eq!(v.minor(), minor);
        assert_eq!(v.patch(), patch);
    }

    #[quickcheck_macros::quickcheck]
    fn version_from_components(major: u16, minor: u16, patch: u16) -> quickcheck::TestResult {
        let v = Version::new(major, minor, patch);
        quickcheck::TestResult::from_bool(
            v.major() == major && v.minor() == minor && v.patch() == patch,
        )
    }
}
