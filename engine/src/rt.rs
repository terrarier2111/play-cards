use std::{fmt::Debug, mem::transmute};

#[derive(Clone, Copy, PartialEq)]
pub struct RtRef {
    ty: RtType,
    val: usize,
}

impl Debug for RtRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RtRef").finish() // FIXME: implement this properly
    }
}

impl RtRef {
    pub const NULL: RtRef = Self {
        ty: RtType::None,
        val: 0,
    };

    pub fn ty(self) -> RtType {
        self.ty
    }

    pub(crate) fn dst(self) -> *mut () {
        self.val as *mut ()
    }

    #[inline]
    pub fn bool(val: bool) -> Self {
        Self {
            ty: RtType::Bool,
            val: val as u8 as usize,
        }
    }

    #[inline]
    pub fn decimal(val: f64) -> Self {
        Self {
            ty: RtType::Decimal,
            val: unsafe { transmute(val) },
        }
    }

    pub fn string(val: Box<String>) -> Self {
        let ptr = Box::into_raw(val);
        Self {
            ty: RtType::String,
            val: ptr as usize,
        }
    }

    pub fn player(val: Player) -> Self {
        Self {
            ty: RtType::Player,
            val: val.0 as usize,
        }
    }

    pub fn inventory(val: CardInventoryRef) -> Self {
        Self {
            ty: RtType::Inventory,
            val: val.0 as usize,
        }
    }

    pub fn function(idx: usize) -> Self {
        Self {
            ty: RtType::Function,
            val: idx,
        }
    }

    pub fn list(val: Box<Vec<RtRef>>) -> Self {
        Self {
            ty: RtType::List,
            val: Box::into_raw(val) as usize,
        }
    }

    pub fn get_player(self) -> Option<Player> {
        match self.ty() {
            RtType::Player => Some(Player(self.dst() as usize as u64)),
            _ => None,
        }
    }

    pub fn get_inventory(self) -> Option<CardInventoryRef> {
        match self.ty() {
            RtType::Inventory => Some({
                let val = self.dst() as usize as u64;
                CardInventoryRef(val)
            }),
            _ => None,
        }
    }

    pub fn get_card(&self) -> Option<CardVal> {
        match self.ty() {
            RtType::Card => Some(CardVal(self.val as u64)),
            _ => None,
        }
    }

    pub fn get_list(&self) -> Option<&Vec<RtRef>> {
        match self.ty() {
            RtType::List => Some(unsafe { &*self.dst().cast::<Vec<RtRef>>() }),
            _ => None,
        }
    }

    pub fn get_list_mut(&self) -> Option<&mut Vec<RtRef>> {
        match self.ty() {
            RtType::List => Some(unsafe { &mut *self.dst().cast::<Vec<RtRef>>() }),
            _ => None,
        }
    }

    pub(crate) unsafe fn get_decimal_directly(self) -> f64 {
        unsafe { transmute(self.val) }
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
                    Err(_) => None,
                }
            }
            _ => None,
        }
    }

    pub(crate) unsafe fn get_bool_directly(self) -> bool {
        unsafe { transmute(self.val as u8) }
    }

    pub fn get_bool(self) -> Option<bool> {
        match self.ty() {
            RtType::Bool => Some(unsafe { self.get_bool_directly() }),
            _ => None,
        }
    }

    pub(crate) unsafe fn get_string_directly(&self) -> &String {
        unsafe { &*self.dst().cast::<String>() }
    }

    pub fn get_string(&self) -> Option<&String> {
        match self.ty() {
            RtType::String => Some(unsafe { self.get_string_directly() }),
            _ => None,
        }
    }

    pub fn get_func_idx(&self) -> Option<usize> {
        match self.ty() {
            RtType::Function => Some(self.val),
            _ => None,
        }
    }

    pub fn to_string(self) -> String {
        match self.ty {
            RtType::Decimal => unsafe { transmute::<_, f64>(self.val) }.to_string(),
            RtType::None => "Null".to_string(),
            RtType::Bool => unsafe { transmute::<_, bool>(self.val as u8) }.to_string(),
            RtType::String => unsafe { (self.val as *const String).as_ref().unwrap() }.clone(),
            RtType::Function => todo!(),
            RtType::List => todo!(),
            RtType::Player => todo!(),
            RtType::Inventory => todo!(),
            RtType::Card => todo!(),
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
#[repr(i8)]
pub enum Ordering {
    Less = -1,
    Equal = 0,
    Greater = 1,
    NotEqual,
}

impl Ordering {
    pub const fn from_std(val: std::cmp::Ordering) -> Self {
        match val {
            std::cmp::Ordering::Less => Self::Less,
            std::cmp::Ordering::Equal => Self::Equal,
            std::cmp::Ordering::Greater => Self::Greater,
        }
    }
}

// this is inlined into rtref using NaN-boxing
#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u64)]
pub enum RtType {
    Decimal = 0,
    None = 1,
    Bool = 2,
    String = 3,
    Function = 4,
    List = 5,
    Player = 6,
    Inventory = 7,
    Card = 8,
}

#[derive(Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct CardVal(u64);

#[derive(Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct Player(u64);

impl Player {
    pub const fn new(idx: u64) -> Self {
        Self(idx)
    }

    pub const fn idx(self) -> u64 {
        self.0
    }
}

pub struct CardInventoryRef(pub u64);

pub struct CardInventory {
    pub slots: u64,
    pub vis: Option<Vec<Player>>,
    pub cards: Vec<CardVal>,
}
