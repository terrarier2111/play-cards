use std::num::NonZeroU64;

#[derive(Clone, Copy)]
pub union NanBox64 {
    pub float: f64,
    tagged: u64,
}

impl NanBox64 {
    #[inline]
    pub const fn new_float(val: f64) -> Self {
        Self { float: val }
    }

    #[inline]
    pub const unsafe fn new_raw_tag(raw_tag: u64) -> Self {
        Self {
            tagged: raw_tag | (u64::MAX & EXP_FIELD_MASK),
        }
    }

    /// # Safety
    /// This is only safe if the non-zero field is filled
    /// with a non-zero value.
    #[inline]
    pub const fn new_tag(tag: TagBuilder) -> Self {
        unsafe { Self::new_raw_tag(tag.0) }
    }

    #[inline]
    pub const fn is_tagged(self) -> bool {
        unsafe { self.tagged & !(SIGN_MASK | ARBITRARY_FIELD_MASK) > EXP_FIELD_MASK }
    }

    /// # Safety
    /// This is only safe if the NanBox is actually tagged
    /// and the tag has its non-zero field filled with a
    /// non-zero value.
    #[inline]
    pub const unsafe fn get_tag(self) -> Tag {
        Tag(self.tagged)
    }
}

#[derive(Clone, Copy)]
pub struct Tag(u64);

impl Tag {
    #[inline]
    pub const fn into_raw(self) -> u64 {
        self.0 & !EXP_FIELD_MASK
    }

    #[inline]
    pub const fn get_val(self) -> u64 {
        self.into_raw() >> 12
    }

    #[inline]
    pub const fn is_sign_pos(self) -> bool {
        self.0 & SIGN_MASK != 0
    }

    #[inline]
    pub const fn sign_raw(self) -> u64 {
        self.0 & SIGN_MASK
    }

    /// only the first 2 bits may contain data
    #[inline]
    pub const fn arbitrary_field(self) -> u64 {
        (self.0 & ARBITRARY_FIELD_MASK) >> ARBITRARY_FIELD_MASK.leading_zeros()
    }

    #[inline]
    pub const fn arbitrary_field_raw(self) -> u64 {
        self.0 & ARBITRARY_FIELD_MASK
    }

    /// only the first 50 bits contain data
    #[inline]
    pub const fn non_zero_field(self) -> NonZeroU64 {
        unsafe {
            NonZeroU64::new_unchecked(
                (self.0 & NON_ZERO_FIELD_MASK) >> NON_ZERO_FIELD_MASK.leading_zeros(),
            )
        }
    }

    #[inline]
    pub const fn non_zero_field_raw(self) -> u64 {
        self.0 & NON_ZERO_FIELD_MASK
    }
}

/// This field is allowed to store 3 arbitrary bits of information in its 3 LSB
pub struct ArbitraryField(u8);
/// This field is allowed to store 50 (not all zero) bits of information in its 50 LSB
pub struct NonZeroField(u64);

const SIGN_MASK: u64 = 1 << 0;
const ARBITRARY_FIELD_MASK: u64 = ((1 << 0) | (1 << 1)) << 12;
const NON_ZERO_FIELD_MASK: u64 = {
    let mut mask = 0;
    let mut i = 0;
    while i < 50 {
        mask |= 1 << (12 + i);
        i += 1;
    }
    mask
};
const EXP_FIELD_MASK: u64 = {
    let mut mask = 0;
    let mut i = 0;
    while i < 11 {
        mask |= 1 << (1 + i);
        i += 1;
    }
    mask
};

pub struct TagBuilder(u64);

impl TagBuilder {
    #[inline]
    pub const unsafe fn invalid() -> Self {
        Self(0)
    }

    #[inline]
    pub const fn new_full_tag(tag: NonZeroU64) -> Self {
        Self(tag.get() << 12)
    }

    #[inline]
    pub const unsafe fn new_raw(tag: u64) -> Self {
        Self(tag)
    }

    #[inline]
    pub const fn sign(mut self, sign: bool) -> Self {
        let sign = sign as u8 as u64;
        self.0 |= sign;
        self.0 &= !(!sign & SIGN_MASK);
        self
    }

    /// the field value is 2 bit-sized and allowed to store arbitrary values.
    #[inline]
    pub const fn arbitrary_field(mut self, field: u64) -> Self {
        self.0 |= field << 12;
        self.0 &= !(!(field << 12) & ARBITRARY_FIELD_MASK);
        self
    }

    /// the field value is 50 bit-sized and allowed to store (non-all zero) values
    #[inline]
    pub const fn non_zero_field(mut self, field: u64) -> Self {
        self.0 |= field << 14;
        self.0 &= !(!(field << 14) & NON_ZERO_FIELD_MASK);
        self
    }
}
