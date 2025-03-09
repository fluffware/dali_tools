// Device commands, opcode byte
pub const OFF: u8 = 0x00;

pub const UP: u8 = 0x01;
pub const DOWN: u8 = 0x02;

pub const STEP_UP: u8 = 0x03;
pub const STEP_DOWN: u8 = 0x04;

pub const RECALL_MAX_LEVEL: u8 = 0x05;
pub const RECALL_MIN_LEVEL: u8 = 0x06;

pub const STEP_DOWN_AND_OFF: u8 = 0x07;
pub const ON_AND_STEP_UP: u8 = 0x08;

pub const ENABLE_DAPC: u8 = 0x09;
pub const GO_TO_LAST_ACTIVE_LEVEL: u8 = 0x0a;

pub const GO_TO_SCENE_0: u8 = 0x10;
pub const GO_TO_SCENE_1: u8 = 0x11;
pub const GO_TO_SCENE_2: u8 = 0x12;
pub const GO_TO_SCENE_3: u8 = 0x13;
pub const GO_TO_SCENE_4: u8 = 0x14;
pub const GO_TO_SCENE_5: u8 = 0x15;
pub const GO_TO_SCENE_6: u8 = 0x16;
pub const GO_TO_SCENE_7: u8 = 0x17;
pub const GO_TO_SCENE_8: u8 = 0x18;
pub const GO_TO_SCENE_9: u8 = 0x19;
pub const GO_TO_SCENE_10: u8 = 0x1a;
pub const GO_TO_SCENE_11: u8 = 0x1b;
pub const GO_TO_SCENE_12: u8 = 0x1c;
pub const GO_TO_SCENE_13: u8 = 0x1d;
pub const GO_TO_SCENE_14: u8 = 0x1e;
pub const GO_TO_SCENE_15: u8 = 0x1f;

pub const RESET: u8 = 0x20;
pub const STORE_ACTUAL_LEVEL_IN_DTR0: u8 = 0x21;
pub const SAVE_PERSISTENT_VARIABLES: u8 = 0x22;
pub const SET_OPERATING_MODE: u8 = 0x23;
pub const RESET_MEMORY_BANK: u8 = 0x24;

pub const IDENTIFY_DEVICE: u8 = 0x25;
pub const SET_MAX_LEVEL: u8 = 0x2a;
pub const SET_MIN_LEVEL: u8 = 0x2b;
pub const SET_SYSTEM_FAILURE_LEVEL: u8 = 0x2c;
pub const SET_POWER_ON_LEVEL: u8 = 0x2d;
pub const SET_FADE_TIME: u8 = 0x2e;
pub const SET_FADE_RATE: u8 = 0x2f;
pub const SET_EXTENDED_FADE_TIME: u8 = 0x30;

pub const SET_SCENE_0: u8 = 0x40;
pub const SET_SCENE_1: u8 = 0x41;
pub const SET_SCENE_2: u8 = 0x42;
pub const SET_SCENE_3: u8 = 0x43;
pub const SET_SCENE_4: u8 = 0x44;
pub const SET_SCENE_5: u8 = 0x45;
pub const SET_SCENE_6: u8 = 0x46;
pub const SET_SCENE_7: u8 = 0x47;
pub const SET_SCENE_8: u8 = 0x48;
pub const SET_SCENE_9: u8 = 0x49;
pub const SET_SCENE_10: u8 = 0x4a;
pub const SET_SCENE_11: u8 = 0x4b;
pub const SET_SCENE_12: u8 = 0x4c;
pub const SET_SCENE_13: u8 = 0x4d;
pub const SET_SCENE_14: u8 = 0x4e;
pub const SET_SCENE_15: u8 = 0x4f;

pub const REMOVE_FROM_SCENE_0: u8 = 0x50;
pub const REMOVE_FROM_SCENE_1: u8 = 0x51;
pub const REMOVE_FROM_SCENE_2: u8 = 0x52;
pub const REMOVE_FROM_SCENE_3: u8 = 0x53;
pub const REMOVE_FROM_SCENE_4: u8 = 0x54;
pub const REMOVE_FROM_SCENE_5: u8 = 0x55;
pub const REMOVE_FROM_SCENE_6: u8 = 0x56;
pub const REMOVE_FROM_SCENE_7: u8 = 0x57;
pub const REMOVE_FROM_SCENE_8: u8 = 0x58;
pub const REMOVE_FROM_SCENE_9: u8 = 0x59;
pub const REMOVE_FROM_SCENE_10: u8 = 0x5a;
pub const REMOVE_FROM_SCENE_11: u8 = 0x5b;
pub const REMOVE_FROM_SCENE_12: u8 = 0x5c;
pub const REMOVE_FROM_SCENE_13: u8 = 0x5d;
pub const REMOVE_FROM_SCENE_14: u8 = 0x5e;
pub const REMOVE_FROM_SCENE_15: u8 = 0x5f;

pub const ADD_TO_GROUP_0: u8 = 0x60;
pub const ADD_TO_GROUP_1: u8 = 0x61;
pub const ADD_TO_GROUP_2: u8 = 0x62;
pub const ADD_TO_GROUP_3: u8 = 0x63;
pub const ADD_TO_GROUP_4: u8 = 0x64;
pub const ADD_TO_GROUP_5: u8 = 0x65;
pub const ADD_TO_GROUP_6: u8 = 0x66;
pub const ADD_TO_GROUP_7: u8 = 0x67;
pub const ADD_TO_GROUP_8: u8 = 0x68;
pub const ADD_TO_GROUP_9: u8 = 0x69;
pub const ADD_TO_GROUP_10: u8 = 0x6a;
pub const ADD_TO_GROUP_11: u8 = 0x6b;
pub const ADD_TO_GROUP_12: u8 = 0x6c;
pub const ADD_TO_GROUP_13: u8 = 0x6d;
pub const ADD_TO_GROUP_14: u8 = 0x6e;
pub const ADD_TO_GROUP_15: u8 = 0x6f;

pub const REMOVE_FROM_GROUP_0: u8 = 0x70;
pub const REMOVE_FROM_GROUP_1: u8 = 0x71;
pub const REMOVE_FROM_GROUP_2: u8 = 0x72;
pub const REMOVE_FROM_GROUP_3: u8 = 0x73;
pub const REMOVE_FROM_GROUP_4: u8 = 0x74;
pub const REMOVE_FROM_GROUP_5: u8 = 0x75;
pub const REMOVE_FROM_GROUP_6: u8 = 0x76;
pub const REMOVE_FROM_GROUP_7: u8 = 0x77;
pub const REMOVE_FROM_GROUP_8: u8 = 0x78;
pub const REMOVE_FROM_GROUP_9: u8 = 0x79;
pub const REMOVE_FROM_GROUP_10: u8 = 0x7a;
pub const REMOVE_FROM_GROUP_11: u8 = 0x7b;
pub const REMOVE_FROM_GROUP_12: u8 = 0x7c;
pub const REMOVE_FROM_GROUP_13: u8 = 0x7d;
pub const REMOVE_FROM_GROUP_14: u8 = 0x7e;
pub const REMOVE_FROM_GROUP_15: u8 = 0x7f;

pub const SET_SHORT_ADDRESS: u8 = 0x80;
pub const ENABLE_WRITE_MEMORY: u8 = 0x81;

pub const QUERY_STATUS: u8 = 0x90;
pub const QUERY_CONTROL_GEAR_PRESENT: u8 = 0x91;
pub const QUERY_LAMP_FAILURE: u8 = 0x92;
pub const QUERY_LAMP_POWER_ON: u8 = 0x93;
pub const QUERY_LIMIT_ERROR: u8 = 0x94;
pub const QUERY_RESET_STATE: u8 = 0x95;
pub const QUERY_MISSING_SHORT_ADDRESS: u8 = 0x96;
pub const QUERY_VERSION_NUMBER: u8 = 0x97;
pub const QUERY_CONTENT_DTR0: u8 = 0x98;
pub const QUERY_DEVICE_TYPE: u8 = 0x99;
pub const QUERY_PHYSICAL_MINIMUM: u8 = 0x9a;
pub const QUERY_POWER_FAILURE: u8 = 0x9b;

pub const QUERY_CONTENT_DTR1: u8 = 0x9c;
pub const QUERY_CONTENT_DTR2: u8 = 0x9d;

pub const QUERY_OPERATING_MODE: u8 = 0x9e;
pub const QUERY_LIGHT_SOURCE_TYPE: u8 = 0x9f;

pub const QUERY_ACTUAL_LEVEL: u8 = 0xa0;
pub const QUERY_MAX_LEVEL: u8 = 0xa1;
pub const QUERY_MIN_LEVEL: u8 = 0xa2;
pub const QUERY_POWER_ON_LEVEL: u8 = 0xa3;
pub const QUERY_SYSTEM_FAILURE_LEVEL: u8 = 0xa4;
pub const QUERY_FADE: u8 = 0xa5;
pub const QUERY_MANUFACTURER_SPECIFIC_MODE: u8 = 0xa6;
pub const QUERY_NEXT_DEVICE_TYPE: u8 = 0xa7;
pub const QUERY_EXTENDED_FADE_TIME: u8 = 0xa8;
pub const QUERY_CONTROL_GEAR_FAILURE: u8 = 0xaa;

pub const QUERY_SCENE_LEVEL_0: u8 = 0xb0;
pub const QUERY_SCENE_LEVEL_1: u8 = 0xb1;
pub const QUERY_SCENE_LEVEL_2: u8 = 0xb2;
pub const QUERY_SCENE_LEVEL_3: u8 = 0xb3;
pub const QUERY_SCENE_LEVEL_4: u8 = 0xb4;
pub const QUERY_SCENE_LEVEL_5: u8 = 0xb5;
pub const QUERY_SCENE_LEVEL_6: u8 = 0xb6;
pub const QUERY_SCENE_LEVEL_7: u8 = 0xb7;
pub const QUERY_SCENE_LEVEL_8: u8 = 0xb8;
pub const QUERY_SCENE_LEVEL_9: u8 = 0xb9;
pub const QUERY_SCENE_LEVEL_10: u8 = 0xba;
pub const QUERY_SCENE_LEVEL_11: u8 = 0xbb;
pub const QUERY_SCENE_LEVEL_12: u8 = 0xbc;
pub const QUERY_SCENE_LEVEL_13: u8 = 0xbd;
pub const QUERY_SCENE_LEVEL_14: u8 = 0xbe;
pub const QUERY_SCENE_LEVEL_15: u8 = 0xbf;

pub const QUERY_GROUPS_0_7: u8 = 0xc0;
pub const QUERY_GROUPS_8_15: u8 = 0xc1;
pub const QUERY_RANDOM_ADDRESS_H: u8 = 0xc2;
pub const QUERY_RANDOM_ADDRESS_M: u8 = 0xc3;
pub const QUERY_RANDOM_ADDRESS_L: u8 = 0xc4;
pub const READ_MEMORY_LOCATION: u8 = 0xc5;

pub const APP_EXT_CMDS_FIRST: u8 = 0xe0;
pub const APP_EXT_CMDS_LAST: u8 = 0xe0;

pub const QUERY_EXTENDED_VERSION_NUMBER: u8 = 0xff;

pub const TERMINATE: u8 = 0xa1;
pub const DTR0: u8 = 0xa3;

pub const INITIALISE: u8 = 0xa5;
pub const INITIALISE_ALL: u8 = 0x00;
pub const INITIALISE_NO_ADDR: u8 = 0xa5;

pub const RANDOMISE: u8 = 0xa7;
pub const COMPARE: u8 = 0xa9;
pub const WITHDRAW: u8 = 0xab;
pub const PING: u8 = 0xad;

pub const SEARCHADDRH: u8 = 0xb1;
pub const SEARCHADDRM: u8 = 0xb3;
pub const SEARCHADDRL: u8 = 0xb5;

pub const PROGRAM_SHORT_ADDRESS: u8 = 0xb7;
pub const VERIFY_SHORT_ADDRESS: u8 = 0xb9;
pub const QUERY_SHORT_ADDRESS: u8 = 0xbb;

pub const ENABLE_DEVICE_TYPE: u8 = 0xc1;
pub const DTR1: u8 = 0xc3;
pub const DTR2: u8 = 0xc5;
pub const WRITE_MEMORY_LOCATION: u8 = 0xc7;
pub const WRITE_MEMORY_LOCATION_NO_REPLY: u8 = 0xc9;
