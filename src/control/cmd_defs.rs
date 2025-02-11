pub struct Command<const ANSWER: bool, const TWICE: bool>(pub [u8; 3]);

impl<const ANSWER: bool, const TWICE: bool> Command<ANSWER, TWICE> {
    const fn new(address: u8, instance: u8, opcode: u8) -> Self {
        Command([address, instance, opcode])
    }
}

pub struct DeviceCommand<const ANSWER: bool, const TWICE: bool> {
    opcode: u8,
}

impl<const ANSWER: bool, const TWICE: bool> DeviceCommand<ANSWER, TWICE> {
    const fn new(opcode: u8) -> Self {
        DeviceCommand { opcode }
    }
    pub fn cmd(&self, device: u8) -> Command<ANSWER, TWICE> {
        Command([device | 1, 0xfe, self.opcode])
    }
}

pub struct InstanceCommand<const ANSWER: bool, const TWICE: bool> {
    opcode: u8,
}

impl<const ANSWER: bool, const TWICE: bool> InstanceCommand<ANSWER, TWICE> {
    const fn new(opcode: u8) -> Self {
        InstanceCommand { opcode }
    }
    pub fn cmd(&self, device: u8, instance: u8) -> Command<ANSWER, TWICE> {
        Command([device | 1, instance, self.opcode])
    }
}

// Special commands with one data byte at the end */
pub struct SpecialDataCommand<const ANSWER: bool, const TWICE: bool> {
    instance: u8,
}

impl<const ANSWER: bool, const TWICE: bool> SpecialDataCommand<ANSWER, TWICE> {
    const fn new(instance: u8) -> Self {
        SpecialDataCommand { instance }
    }
    pub fn cmd(&self, data: u8) -> Command<ANSWER, TWICE> {
        Command([0xc1, self.instance, data])
    }
}

// Special commands with two data bytes at the end */
pub struct SpecialData2Command<const ANSWER: bool, const TWICE: bool> {
    address: u8,
}

impl<const ANSWER: bool, const TWICE: bool> SpecialData2Command<ANSWER, TWICE> {
    const fn new(address: u8) -> Self {
        SpecialData2Command { address }
    }
    pub fn cmd(&self, data1: u8, data2: u8) -> Command<ANSWER, TWICE> {
        Command([self.address, data1, data2])
    }
}

macro_rules! dev_cmd_def {
    ($sym: ident, $opcode: expr) => {
        pub const $sym: DeviceCommand<false, false> = DeviceCommand::new($opcode);
    };
    ($sym: ident, $opcode: expr, Answer) => {
        pub const $sym: DeviceCommand<true, false> = DeviceCommand::new($opcode);
    };
    ($sym: ident, $opcode: expr, Twice) => {
        pub const $sym: DeviceCommand<false, true> = DeviceCommand::new($opcode);
    };
}

macro_rules! inst_cmd_def {
    ($sym: ident, $opcode: expr) => {
        pub const $sym: InstanceCommand<false, false> = InstanceCommand::new($opcode);
    };
    ($sym: ident,  $opcode: expr, Answer) => {
        pub const $sym: InstanceCommand<true, false> = InstanceCommand::new($opcode);
    };
    ($sym: ident,   $opcode: expr, Twice) => {
        pub const $sym: InstanceCommand<false, true> = InstanceCommand::new($opcode);
    };
}

macro_rules! special_data_cmd_def {
    ($sym: ident, $instance: expr) => {
        pub const $sym: SpecialDataCommand<false, false> = SpecialDataCommand::new($instance);
    };
    ($sym: ident, $instance: expr, Answer) => {
        pub const $sym: SpecialDataCommand<true, false> = SpecialDataCommand::new($instance);
    };
    ($sym: ident, $instance: expr, Twice) => {
        pub const $sym: SpecialDataCommand<false, true> = SpecialDataCommand::new($instance);
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

pub const TERMINATE: Command<false, false> = Command::new(0xc1, 0x00, 0x00);

special_data_cmd_def!(INITIALISE, 0x01, Twice);
pub const RANDOMISE: Command<false, true> = Command::new(0xc1, 0x02, 0x00);
pub const COMPARE: Command<true, false> = Command::new(0xc1, 0x03, 0x00);
pub const WITHDRAW: Command<false, false> = Command::new(0xc1, 0x04, 0x00);
special_data_cmd_def!(SEARCHADDRH, 0x05);
special_data_cmd_def!(SEARCHADDRM, 0x06);
special_data_cmd_def!(SEARCHADDRL, 0x07);
special_data_cmd_def!(PROGRAM_SHORT_ADDRESS, 0x08);
special_data_cmd_def!(VERIFY_SHORT_ADDRESS, 0x09, Answer);
pub const QUERY_SHORT_ADDRESS: Command<true, false> = Command::new(0xc1, 0x0a, 0x00);
special_data_cmd_def!(WRITE_MEMORY_LOCATION, 0x20, Answer);
special_data_cmd_def!(WRITE_MEMORY_LOCATION_NO_REPLY, 0x21);
special_data_cmd_def!(DTR0, 0x30);
special_data_cmd_def!(DTR1, 0x31);
special_data_cmd_def!(DTR2, 0x32);
special_data_cmd_def!(SEND_TESTFRAME, 0x33);
pub const DIRECT_WRITE_MEMORY: SpecialData2Command<true, false> = SpecialData2Command::new(0xc5);
pub const DTR1_DTR0: SpecialData2Command<false, false> = SpecialData2Command::new(0xc7);
pub const DTR2_DTR1: SpecialData2Command<false, false> = SpecialData2Command::new(0xc9);
