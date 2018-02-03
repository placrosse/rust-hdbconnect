use protocol::lowlevel::parts::option_part::OptionPart;
use protocol::lowlevel::parts::option_part::OptionId;
// use protocol::lowlevel::parts::option_value::OptionValue;

use std::u8;

// An Options part that is used for describing the connection's capabilities.
pub type FetchOptions = OptionPart<FetchOptionsId>;

impl FetchOptions {
    // pub fn set_foo(mut self, b: bool) -> FetchOptions {
    //     self.insert(FetchOptionsId::Foo, OptionValue::BOOLEAN(b));
    //     self
    // }
}


#[derive(Debug, Eq, PartialEq, Hash)]
pub enum FetchOptionsId {
    ResultsetPosition, // 1 // INT // Position for Fetch
    __Unexpected__,
}

impl OptionId<FetchOptionsId> for FetchOptionsId {
    fn to_u8(&self) -> u8 {
        match *self {
            FetchOptionsId::ResultsetPosition => 1,
            FetchOptionsId::__Unexpected__ => u8::MAX,
        }
    }

    fn from_u8(val: u8) -> FetchOptionsId {
        match val {
            1 => FetchOptionsId::ResultsetPosition,
            val => {
                warn!("Unsupported value for FetchOptionsId received: {}", val);
                FetchOptionsId::__Unexpected__
            }
        }
    }
}