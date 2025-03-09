use crate::common::address::Short;
use crate::common::cmd_defs::AddressByte;

pub struct Command<const ANSWER: bool, const TWICE: bool>(pub [u8; 3]);

macro_rules! cmd_type {
    () => {Command<false,false>};
    (Answer) => {Command<true,false>};
    (Twice) => {Command<false,true>};
}

macro_rules! dev_cmd_def {
   ($sym: ident, $opcode: expr $(,$attr: ident)?) => {
       #[allow(non_snake_case)]
       #[inline(always)]
       pub fn $sym<A>(addr: A) -> cmd_type!($($attr)?)
       where
           A: Into<AddressByte>,
       {
            Command([addr.into().0, 0xfe, $opcode])
        }
    };
}

macro_rules! inst_cmd_def {
    ($sym: ident, $opcode: expr $(,$attr: ident)?) => {
	#[allow(non_snake_case)]
	#[inline(always)]
        pub fn $sym<A>(addr: A, inst: u8) -> cmd_type!($($attr)?)
        where
            A: Into<AddressByte>,
        {
            Command([addr.into().0, inst, $opcode])
        }
    };
}
macro_rules! special_cmd_def {
    ($sym: ident, $inst: expr, $opcode: expr $(,$attr: ident)?) => {
	#[allow(non_snake_case)]
	#[inline(always)]
        pub fn $sym() -> cmd_type!($($attr)?)
        {
            Command([0xc1, $inst, $opcode])
        }
    };
}
macro_rules! special_data_cmd_def {
     ($sym: ident, $opcode: expr $(,$attr: ident)?) => {
	#[allow(non_snake_case)]
	#[inline(always)]
        pub fn $sym(data: u8) -> cmd_type!($($attr)?)
        {
            Command([0xc1, $opcode, data])
        }
    };
}

dev_cmd_def!(IDENTIFY_DEVICE, 0x00, Twice);
dev_cmd_def!(RESET_POWER_CYCLE_SEEN, 0x01, Twice);
dev_cmd_def!(RESET, 0x10, Twice);
dev_cmd_def!(RESET_MEMORY_BANK, 0x11, Twice);
dev_cmd_def!(SET_SHORT_ADDRESS, 0x14, Twice);
dev_cmd_def!(ENABLE_WRITE_MEMORY, 0x15, Twice);
dev_cmd_def!(ENABLE_APPLICATION_CONTROLLER, 0x16, Twice);
dev_cmd_def!(DISABLE_APPLICATION_CONTROLLER, 0x17, Twice);
dev_cmd_def!(SET_OPERATING_MODE, 0x18, Twice);
dev_cmd_def!(ADD_TO_DEVICE_GROUPS_0_15, 0x19, Twice);
dev_cmd_def!(ADD_TO_DEVICE_GROUPS_16_31, 0x1a, Twice);
dev_cmd_def!(REMOVE_FROM_DEVICE_GROUPS_0_15, 0x1b, Twice);
dev_cmd_def!(REMOVE_FROM_DEVICE_GROUPS_16_31, 0x1c, Twice);
dev_cmd_def!(START_QUIESCENT_MODE, 0x1d, Twice);
dev_cmd_def!(STOP_QUIESCENT_MODE, 0x1e, Twice);
dev_cmd_def!(ENABLE_POWER_CYCLE_NOTIFICATION, 0x1f, Twice);
dev_cmd_def!(DISABLE_POWER_CYCLE_NOTIFICATION, 0x20, Twice);
dev_cmd_def!(SAVE_PERSISTENT_VARIABLES, 0x21, Twice);
dev_cmd_def!(QUERY_DEVICE_STATUS, 0x30, Answer);
dev_cmd_def!(QUERY_APPLICATION_CONTROLLER_ERROR, 0x31, Answer);
dev_cmd_def!(QUERY_INPUT_DEVICE_ERROR, 0x32, Answer);
dev_cmd_def!(QUERY_MISSING_SHORT_ADDRESS, 0x33, Answer);
dev_cmd_def!(QUERY_VERSION_NUMBER, 0x34, Answer);
dev_cmd_def!(QUERY_NUMBER_OF_INSTANCES, 0x35, Answer);
dev_cmd_def!(QUERY_CONTENT_DTR0, 0x36, Answer);
dev_cmd_def!(QUERY_CONTENT_DTR1, 0x37, Answer);
dev_cmd_def!(QUERY_CONTENT_DTR2, 0x38, Answer);
dev_cmd_def!(QUERY_RANDOM_ADDRESS_H, 0x39, Answer);
dev_cmd_def!(QUERY_RANDOM_ADDRESS_M, 0x3a, Answer);
dev_cmd_def!(QUERY_RANDOM_ADDRESS_L, 0x3b, Answer);
dev_cmd_def!(READ_MEMORY_LOCATION, 0x3c, Answer);
dev_cmd_def!(QUERY_APPLICATION_CONTROL_ENABLED, 0x3d, Answer);
dev_cmd_def!(QUERY_OPERATING_MODE, 0x3e, Answer);
dev_cmd_def!(QUERY_MANUFACTURER_SPECIFIC_MODE, 0x3f, Answer);
dev_cmd_def!(QUERY_QUIESCENT_MODE, 0x40, Answer);
dev_cmd_def!(QUERY_DEVICE_GROUPS_0_7, 0x41, Answer);
dev_cmd_def!(QUERY_DEVICE_GROUPS_8_15, 0x42, Answer);
dev_cmd_def!(QUERY_DEVICE_GROUPS_16_23, 0x43, Answer);
dev_cmd_def!(QUERY_DEVICE_GROUPS_24_31, 0x44, Answer);
dev_cmd_def!(QUERY_POWER_CYCLE_NOTIFICATION, 0x45, Answer);
dev_cmd_def!(QUERY_DEVICE_CAPABILITIES, 0x46, Answer);
dev_cmd_def!(QUERY_EXTENDED_VERSION_NUMBER, 0x47, Answer);
dev_cmd_def!(QUERY_RESET_STATE, 0x48, Answer);

inst_cmd_def!(SET_EVENT_PRIORITY, 0x61, Twice);
inst_cmd_def!(ENABLE_INSTANCE, 0x62, Twice);
inst_cmd_def!(DISABLE_INSTANCE, 0x63, Twice);
inst_cmd_def!(SET_PRIMARY_INSTANCE_GROUP, 0x64, Twice);
inst_cmd_def!(SET_INSTANCE_GROUP_1, 0x65, Twice);
inst_cmd_def!(SET_INSTANCE_GROUP_2, 0x66, Twice);
inst_cmd_def!(SET_EVENT_SCHEME, 0x67, Twice);
inst_cmd_def!(SET_EVENT_FILTER, 0x68, Twice);

inst_cmd_def!(QUERY_INSTANCE_TYPE, 0x80, Answer);
inst_cmd_def!(QUERY_RESOLUTION, 0x81, Answer);
inst_cmd_def!(QUERY_INSTANCE_ERROR, 0x82, Answer);
inst_cmd_def!(QUERY_INSTANCE_STATUS, 0x83, Answer);
inst_cmd_def!(QUERY_EVENT_PRIORITY, 0x84, Answer);
inst_cmd_def!(QUERY_INSTANCE_ENABLED, 0x86, Answer);
inst_cmd_def!(QUERY_PRIMARY_INSTANCE_GROUP, 0x88, Answer);
inst_cmd_def!(QUERY_INSTANCE_GROUP_1, 0x89, Answer);
inst_cmd_def!(QUERY_INSTANCE_GROUP_2, 0x8a, Answer);
inst_cmd_def!(QUERY_EVENT_SCHEME, 0x8b, Answer);
inst_cmd_def!(QUERY_INPUT_VALUE, 0x8c, Answer);
inst_cmd_def!(QUERY_INPUT_VALUE_LATCH, 0x8d, Answer);
inst_cmd_def!(QUERY_FEATURE_TYPE, 0x8e, Answer);
inst_cmd_def!(QUERY_NEXT_FEATURE_TYPE, 0x8f, Answer);
inst_cmd_def!(QUERY_EVENT_FILTER_0_7, 0x90, Answer);
inst_cmd_def!(QUERY_EVENT_FILTER_8_15, 0x91, Answer);
inst_cmd_def!(QUERY_EVENT_FILTER_16_23, 0x92, Answer);

special_cmd_def!(TERMINATE, 0x00, 0x00);

#[allow(non_snake_case)]
#[inline(always)]
pub fn INITIALISE_ADDR(addr: Short) -> Command<false, true> {
    Command([0xc1, 0x01, addr.value()])
}
special_cmd_def!(INITIALISE_NO_ADDR, 0x01, 0x7f);
special_cmd_def!(INITIALISE_ALL, 0x01, 0xff);
special_cmd_def!(RANDOMISE, 0x02, 0x00, Twice);
special_cmd_def!(COMPARE, 0x03, 0x00, Answer);
special_cmd_def!(WITHDRAW, 0x04, 0x00);
special_data_cmd_def!(SEARCHADDRH, 0x05);
special_data_cmd_def!(SEARCHADDRM, 0x06);
special_data_cmd_def!(SEARCHADDRL, 0x07);

#[allow(non_snake_case)]
#[inline(always)]
pub fn PROGRAM_SHORT_ADDRESS<A>(addr: A) -> Command<false, false>
where
    A: Into<AddressByte>,
{
    Command([0xc1, 0x08, addr.into().0 >> 1])
}

#[allow(non_snake_case)]
#[inline(always)]
pub fn VERIFY_SHORT_ADDRESS<A>(addr: A) -> Command<true, false>
where
    A: Into<AddressByte>,
{
    Command([0xc1, 0x09, addr.into().0 >> 1])
}
special_cmd_def!(QUERY_SHORT_ADDRESS, 0x0a, 0x00, Answer);
special_data_cmd_def!(WRITE_MEMORY_LOCATION, 0x20, Answer);
special_data_cmd_def!(WRITE_MEMORY_LOCATION_NO_REPLY, 0x21);
special_data_cmd_def!(DTR0, 0x30);
special_data_cmd_def!(DTR1, 0x31);
special_data_cmd_def!(DTR2, 0x32);
special_data_cmd_def!(SEND_TESTFRAME, 0x33);

#[allow(non_snake_case)]
#[inline(always)]
pub fn DIRECT_WRITE_MEMORY(offset: u8, data: u8) -> Command<true, false> {
    Command([0xc5, offset, data])
}

#[allow(non_snake_case)]
#[inline(always)]
pub fn DTR1_DTR0(data1: u8, data0: u8) -> Command<true, false> {
    Command([0xc7, data1, data0])
}

#[allow(non_snake_case)]
#[inline(always)]
pub fn DTR2_DTR1(data2: u8, data1: u8) -> Command<true, false> {
    Command([0xc9, data2, data1])
}
