use std::{mem::transmute, num::NonZeroU64};

use thin_string::ThinString;
use thin_vec::ThinVec;

use crate::nan_box::{NanBox64, TagBuilder};

#[derive(Clone, Copy, PartialEq)]
pub struct RtRef(NanBox64);

impl RtRef {
    const TY_MASK: u64 = (1 << 3) - 1;
    const VAL_SHIFT: u64 = 3;
    pub const NULL: RtRef = Self(NanBox64::new_tag(TagBuilder::new_full_tag(unsafe {
        NonZeroU64::new_unchecked(RtType::None as u64)
    })));

    pub fn ty(self) -> RtType {
        if !self.0.is_tagged() {
            return RtType::Decimal;
        }
        unsafe { transmute(self.0.get_tag().get_val() & Self::TY_MASK) }
    }

    pub(crate) fn dst(self) -> *mut () {
        (unsafe { self.0.get_tag().get_val() } >> Self::VAL_SHIFT) as *mut ()
    }

    #[inline]
    pub fn bool(val: bool) -> Self {
        Self(NanBox64::new_tag(TagBuilder::new_full_tag(unsafe {
            NonZeroU64::new_unchecked(
                (RtType::Bool as u64) | ((val as u8 as u64) << Self::VAL_SHIFT),
            )
        })))
    }

    #[inline]
    pub fn decimal(val: f64) -> Self {
        Self(NanBox64::new_float(val))
    }

    pub fn string(val: ThinString) -> Self {
        Self(NanBox64::new_tag(TagBuilder::new_full_tag()))
    }

    pub fn get_player(self) -> Option<Player> {
        match self.ty() {
            RtType::Player => Some(Player(self.dst() as usize as u64)),
            _ => None,
        }
    }

    pub fn get_inventory(self) -> Option<CardInventory> {
        match self.ty() {
            RtType::Inventory => Some({
                let val = self.dst() as usize as u64;
                if val & CardInventory::PLAYER_MARKER != 0 {
                    CardInventory::Player(val & !CardInventory::PLAYER_MARKER)
                } else {
                    CardInventory::Other(val)
                }
            }),
            _ => None,
        }
    }

    pub fn get_cards(&self) -> Option<&ThinVec<CardVal>> {
        match self.ty() {
            RtType::Cards => Some(unsafe { &*self.dst().cast::<ThinVec<CardVal>>() }),
            _ => None,
        }
    }

    pub(crate) unsafe fn get_decimal_directly(self) -> f64 {
        unsafe { self.0.float }
    }

    pub fn get_decimal(self) -> Option<f64> {
        match self.ty() {
            RtType::Decimal => Some(unsafe { self.get_decimal_directly() }),
            RtType::Bool => Some(if unsafe { self.get_bool_directly() } {
                1.0
            } else {
                0.0
            }),
            RtType::String => {
                let val = unsafe { self.get_string_directly() };
                match val.parse::<f64>() {
                    Ok(val) => Some(val),
                    Err(_) => todo!(),
                }
            }
            _ => None,
        }
    }

    pub(crate) unsafe fn get_bool_directly(self) -> bool {
        unsafe { transmute((self.0.get_tag().get_val() >> RtRef::VAL_SHIFT) as u8) }
    }

    pub fn get_bool(self) -> Option<bool> {
        match self.ty() {
            RtType::Bool => Some(unsafe { self.get_bool_directly() }),
            _ => None,
        }
    }

    pub(crate) unsafe fn get_string_directly(&self) -> &ThinString {
        unsafe { &*self.dst().cast::<ThinString>() }
    }

    pub fn get_string(&self) -> Option<&ThinString> {
        match self.ty() {
            RtType::String => Some(unsafe { self.get_string_directly() }),
            _ => None,
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum Ordering {
    Less,
    Equal,
    NotEqual,
    Greater,
}

pub struct RtVal<T>(T);

// this is inlined into rtref using NaN-boxing
#[derive(Clone, Copy, PartialEq)]
#[repr(u64)]
pub enum RtType {
    Decimal,
    None = 1,
    Bool = 2,
    String = 3,
    Player = 4,
    Inventory = 5,
    Cards = 6,
}

#[derive(Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct CardVal(u64);

#[derive(Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct Player(u64);

pub enum Visibility {
    None,
    Select(Vec<usize>),
    All,
}

pub enum CardInventory {
    Player(u64),
    Other(u64),
}

impl CardInventory {
    const PLAYER_MARKER: u64 = 1 << (u64::BITS - 1);
}
