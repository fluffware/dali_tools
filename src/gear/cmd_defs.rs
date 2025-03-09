use crate::common::cmd_defs::AddressByte;

pub struct Command<const ANSWER: bool, const TWICE: bool>(pub [u8; 2]);

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
            Command([addr.into().0, $opcode])
        }
    };
}

macro_rules! offset_cmd_def {
    ($sym: ident, $opcode: expr $(,$attr: ident)?) => {
        #[allow(non_snake_case)]
        #[inline(always)]
        pub fn $sym<A>(addr: A, offset: u8) -> cmd_type!($($attr)?)
        where
            A: Into<AddressByte>,
        {
            Command([addr.into().0, $opcode + offset])
        }
    };
}

macro_rules! special_cmd_def {
    ($sym: ident, $byte1: expr, $byte2: expr $(,$attr: ident)?) => {
        #[allow(non_snake_case)]
        #[inline(always)]
        pub const fn $sym() -> cmd_type!($($attr)?) {
            Command([$byte1, $byte2])
        }
    };
}

macro_rules! special_data_cmd_def {
    ($sym: ident, $byte1: expr $(,$attr: ident)?) => {
        #[allow(non_snake_case)]
        #[inline(always)]
        pub const fn $sym(data: u8) ->cmd_type!($($attr)?) {
            Command([$byte1, data])
        }
    };
}

#[allow(non_snake_case)]
#[inline(always)]
pub fn DAPC<A>(addr: A, level: u8) -> Command<false, true>
where
    A: Into<AddressByte>,
{
    Command([addr.into().0 & 0xfe, level])
}

dev_cmd_def!(OFF, 0x00);
dev_cmd_def!(UP, 0x01);
dev_cmd_def!(DOWN, 0x02);

dev_cmd_def!(STEP_UP, 0x03);
dev_cmd_def!(STEP_DOWN, 0x04);

dev_cmd_def!(RECALL_MAX_LEVEL, 0x05);
dev_cmd_def!(RECALL_MIN_LEVEL, 0x06);

dev_cmd_def!(STEP_DOWN_AND_OFF, 0x07);
dev_cmd_def!(ON_AND_STEP_UP, 0x08);

dev_cmd_def!(ENABLE_DAPC, 0x09);
dev_cmd_def!(GO_TO_LAST_ACTIVE_LEVEL, 0x0a);

offset_cmd_def!(GOTO_SCENE, 0x10);

dev_cmd_def!(RESET, 0x20, Twice);
dev_cmd_def!(STORE_ACTUAL_LEVEL_IN_DTR0, 0x21, Twice);
dev_cmd_def!(SAVE_PERSISTENT_VARIABLES, 0x22, Twice);
dev_cmd_def!(SET_OPERATING_MODE, 0x23, Twice);
dev_cmd_def!(RESET_MEMORY_BANK, 0x24, Twice);

dev_cmd_def!(IDENTIFY_DEVICE, 0x25, Twice);
dev_cmd_def!(SET_MAX_LEVEL, 0x2a, Twice);
dev_cmd_def!(SET_MIN_LEVEL, 0x2b, Twice);
dev_cmd_def!(SET_SYSTEM_FAILURE_LEVEL, 0x2c, Twice);
dev_cmd_def!(SET_POWER_ON_LEVEL, 0x2d, Twice);
dev_cmd_def!(SET_FADE_TIME, 0x2e, Twice);
dev_cmd_def!(SET_FADE_RATE, 0x2f, Twice);
dev_cmd_def!(SET_EXTENDED_FADE_TIME, 0x30, Twice);

offset_cmd_def!(SET_SCENE, 0x40, Twice);
offset_cmd_def!(REMOVE_FROM_SCENE, 0x50, Twice);

offset_cmd_def!(ADD_TO_GROUP, 0x60, Twice);
offset_cmd_def!(REMOVE_FROM_GROUP, 0x70, Twice);

dev_cmd_def!(SET_SHORT_ADDRESS, 0x80, Twice);
dev_cmd_def!(ENABLE_WRITE_MEMORY, 0x81, Twice);

dev_cmd_def!(QUERY_STATUS, 0x90, Answer);
dev_cmd_def!(QUERY_CONTROL_GEAR_PRESENT, 0x91, Answer);
dev_cmd_def!(QUERY_LAMP_FAILURE, 0x92, Answer);
dev_cmd_def!(QUERY_LAMP_POWER_ON, 0x93, Answer);
dev_cmd_def!(QUERY_LIMIT_ERROR, 0x94, Answer);
dev_cmd_def!(QUERY_RESET_STATE, 0x95, Answer);
dev_cmd_def!(QUERY_MISSING_SHORT_ADDRESS, 0x96, Answer);
dev_cmd_def!(QUERY_VERSION_NUMBER, 0x97, Answer);
dev_cmd_def!(QUERY_CONTENT_DTR0, 0x98, Answer);
dev_cmd_def!(QUERY_DEVICE_TYPE, 0x99, Answer);
dev_cmd_def!(QUERY_PHYSICAL_MINIMUM, 0x9a, Answer);
dev_cmd_def!(QUERY_POWER_FAILURE, 0x9b, Answer);

dev_cmd_def!(QUERY_CONTENT_DTR1, 0x9c, Answer);
dev_cmd_def!(QUERY_CONTENT_DTR2, 0x9d, Answer);

dev_cmd_def!(QUERY_OPERATING_MODE, 0x9e, Answer);
dev_cmd_def!(QUERY_LIGHT_SOURCE_TYPE, 0x9f, Answer);

dev_cmd_def!(QUERY_ACTUAL_LEVEL, 0xa0, Answer);
dev_cmd_def!(QUERY_MAX_LEVEL, 0xa1, Answer);
dev_cmd_def!(QUERY_MIN_LEVEL, 0xa2, Answer);
dev_cmd_def!(QUERY_POWER_ON_LEVEL, 0xa3, Answer);
dev_cmd_def!(QUERY_SYSTEM_FAILURE_LEVEL, 0xa4, Answer);
dev_cmd_def!(QUERY_FADE, 0xa5, Answer);
dev_cmd_def!(QUERY_MANUFACTURER_SPECIFIC_MODE, 0xa6, Answer);
dev_cmd_def!(QUERY_NEXT_DEVICE_TYPE, 0xa7, Answer);
dev_cmd_def!(QUERY_EXTENDED_FADE_TIME, 0xa8, Answer);
dev_cmd_def!(QUERY_CONTROL_GEAR_FAILURE, 0xaa, Answer);

offset_cmd_def!(QUERY_SCENE_LEVEL, 0xb0, Answer);

dev_cmd_def!(QUERY_GROUPS_0_7, 0xc0, Answer);
dev_cmd_def!(QUERY_GROUPS_8_15, 0xc1, Answer);
dev_cmd_def!(QUERY_RANDOM_ADDRESS_H, 0xc2, Answer);
dev_cmd_def!(QUERY_RANDOM_ADDRESS_M, 0xc3, Answer);
dev_cmd_def!(QUERY_RANDOM_ADDRESS_L, 0xc4, Answer);
dev_cmd_def!(READ_MEMORY_LOCATION, 0xc5, Answer);

dev_cmd_def!(QUERY_EXTENDED_VERSION_NUMBER, 0xff, Answer);

special_cmd_def!(TERMINATE, 0xa1, 0x00);

#[allow(non_snake_case)]
#[inline(always)]
pub fn INITIALISE_ADDR<A>(addr: A) -> Command<false, true>
where
    A: Into<AddressByte>,
{
    Command([0xa5, addr.into().0])
}

special_cmd_def!(INITIALISE_ALL, 0xa5, 0x00, Twice);
special_cmd_def!(INITIALISE_NO_ADDR, 0xa5, 0xff, Twice);
special_cmd_def!(RANDOMISE, 0xa7, 0x00, Twice);
special_cmd_def!(COMPARE, 0xa9, 0x00, Answer);
                                             special_cmd_def!(WITHDRAW, 0xab, 0x00);
special_cmd_def!(PING, 0xad, 0x00);

special_data_cmd_def!(SEARCHADDRH, 0xb1);
special_data_cmd_def!(SEARCHADDRM, 0xb3);
special_data_cmd_def!(SEARCHADDRL, 0xb5);

#[allow(non_snake_case)]
#[inline(always)]
pub fn PROGRAM_SHORT_ADDRESS<A>(addr: A) -> Command<false, false>
where
    A: Into<AddressByte>,
{
    Command([0xb7, addr.into().0])
}

#[allow(non_snake_case)]
#[inline(always)]
pub fn VERIFY_SHORT_ADDRESS<A>(addr: A) -> Command<true, false>
where
    A: Into<AddressByte>,
{
    Command([0xb9, addr.into().0])
}

special_cmd_def!(QUERY_SHORT_ADDRESS, 0xbb, 0x00, Answer);
special_data_cmd_def!(ENABLE_DEVICE_TYPE, 0xc1);

special_data_cmd_def!(DTR0, 0xa3);
special_data_cmd_def!(DTR1, 0xc3);
special_data_cmd_def!(DTR2, 0xc5);

special_data_cmd_def!(WRITE_MEMORY_LOCATION, 0xc7, Answer);
special_data_cmd_def!(WRITE_MEMORY_LOCATION_NO_REPLY, 0xc9);
