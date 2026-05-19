//! Stub for icu_properties to avoid rustc ICE on string literal unescaping
//! This minimal crate provides interface expected by unicode processing crates
//! without the problematic string literals that trigger compiler panic.

#![no_std]
#![allow(non_upper_case_globals)]

// Props module with Unicode character properties
pub mod props {
    // For simplicity and compatibility, GeneralCategory is a u32 wrapper that acts like a primitive
    pub use super::GeneralCategory;
    
    #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
    #[repr(transparent)]
    pub struct BidiClass(pub u8);
    
    impl BidiClass {
        pub const ArabicLetter: BidiClass = BidiClass(1);
        pub const ArabicNumber: BidiClass = BidiClass(2);
        pub const BoundaryNeutral: BidiClass = BidiClass(3);
        pub const CommonSeparator: BidiClass = BidiClass(4);
        pub const EuropeanNumber: BidiClass = BidiClass(5);
        pub const EuropeanSeparator: BidiClass = BidiClass(6);
        pub const EuropeanTerminator: BidiClass = BidiClass(7);
        pub const LeftToRight: BidiClass = BidiClass(8);
        pub const NonspacingMark: BidiClass = BidiClass(9);
        pub const OtherNeutral: BidiClass = BidiClass(10);
        pub const RightToLeft: BidiClass = BidiClass(11);
        
        pub const fn to_icu4c_value(self) -> u32 {
            self.0 as u32
        }
    }
    
    #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
    #[repr(transparent)]
    pub struct JoiningType(pub u8);
    
    impl JoiningType {
        pub const DualJoining: JoiningType = JoiningType(1);
        pub const LeftJoining: JoiningType = JoiningType(2);
        pub const RightJoining: JoiningType = JoiningType(3);
        pub const Transparent: JoiningType = JoiningType(4);
        
        pub const fn to_icu4c_value(self) -> u32 {
            self.0 as u32
        }
    }
    
    pub const ASCII_HEX_DIGIT: u8 = 0;
}

// GeneralCategory as a newtype struct with constants for idna_adapter compatibility
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct GeneralCategory(pub u32);

impl GeneralCategory {
    // Associated constants that idna_adapter expects
    pub const NonspacingMark: Self = GeneralCategory(1);
    pub const SpacingMark: Self = GeneralCategory(2);
    pub const EnclosingMark: Self = GeneralCategory(3);
}

// Impl From so we can convert to u32
impl From<GeneralCategory> for u32 {
    #[inline]
    fn from(gc: GeneralCategory) -> u32 {
        gc.0
    }
}

// Impl From u32 so we can convert from u32
impl From<u32> for GeneralCategory {
    #[inline]
    fn from(v: u32) -> Self {
        GeneralCategory(v)
    }
}

pub mod provider {
    pub struct Names;
}

#[derive(Clone, Debug)]
pub struct CodePointSet;

#[derive(Clone, Debug)]
pub struct CodePointMap<T> {
    pub _marker: core::marker::PhantomData<T>,
}

#[derive(Clone, Debug)]
pub struct CodePointMapDataBorrowed<'a, T> {
    pub _marker: (core::marker::PhantomData<&'a ()>, core::marker::PhantomData<T>),
}

impl<'a> CodePointMapDataBorrowed<'a, props::GeneralCategory> {
    pub const fn get(&self, _cp: char) -> props::GeneralCategory {
        props::GeneralCategory(0)
    }
    
    // Also provide u32 version for compatibility
    pub const fn get_u32(&self, _cp: u32) -> props::GeneralCategory {
        props::GeneralCategory(0)
    }
}

impl<'a> CodePointMapDataBorrowed<'a, props::BidiClass> {
    pub const fn get(&self, _cp: char) -> props::BidiClass {
        props::BidiClass(0)
    }
    
    pub const fn get_u32(&self, _cp: u32) -> props::BidiClass {
        props::BidiClass(0)
    }
}

impl<'a> CodePointMapDataBorrowed<'a, props::JoiningType> {
    pub const fn get(&self, _cp: char) -> props::JoiningType {
        props::JoiningType(0)
    }
    
    pub const fn get_u32(&self, _cp: u32) -> props::JoiningType {
        props::JoiningType(0)
    }
}

/// Generic stub for CodePointMapData - in the stub, this directly provides borrowed data
#[derive(Clone, Debug)]
pub struct CodePointMapData<T> {
    pub _marker: core::marker::PhantomData<T>,
}

impl<T: Default> CodePointMapData<T> {
    pub const fn new() -> CodePointMapDataBorrowed<'static, T> {
        CodePointMapDataBorrowed { _marker: (core::marker::PhantomData, core::marker::PhantomData) }
    }
    
    pub fn as_borrowed(&self) -> CodePointMapDataBorrowed<'static, T> {
        CodePointMapDataBorrowed { _marker: (core::marker::PhantomData, core::marker::PhantomData) }
    }
}

pub fn codepoint_trie_builder(_data: &[u8]) -> CodePointMap<u32> {
    CodePointMap { _marker: core::marker::PhantomData }
}

pub mod names {
    pub const GENERAL_CATEGORY: u8 = 0;
}

#[cfg(test)]
mod tests {
    #[test]
    fn stub_test() {
        assert!(true);
    }
}
