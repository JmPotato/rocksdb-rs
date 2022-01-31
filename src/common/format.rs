use crate::common::DISABLE_GLOBAL_SEQUENCE_NUMBER;
use crate::util::{decode_fixed_uint64, extract_user_key};

pub enum ValueType {
    TypeDeletion = 0x0,
    TypeValue = 0x1,
    TypeMerge = 0x2,
    TypeLogData = 0x3,              // WAL only.
    TypeColumnFamilyDeletion = 0x4, // WAL only.
    TypeColumnFamilyValue = 0x5,    // WAL only.
    TypeColumnFamilyMerge = 0x6,    // WAL only.

    TypeColumnFamilyRangeDeletion = 0xE, // WAL only.
    TypeRangeDeletion = 0xF,             // meta block
    TypeColumnFamilyBlobIndex = 0x10,    // Blob DB only
    TypeBlobIndex = 0x11,                // Blob DB only
    MaxValue = 0x7F,                     // Not used for storing records.
}

#[derive(Default, Clone)]
pub struct Slice {
    pub offset: usize,
    pub limit: usize,
}

pub const VALUE_TYPE_FOR_SEEK: u8 = ValueType::TypeBlobIndex as u8;

pub fn pack_sequence_and_type(seq: u64, t: u8) -> u64 {
    return (seq << 8) | t as u64;
}

pub fn extract_internal_key_footer(key: &[u8]) -> u64 {
    unsafe { u64::from_le_bytes(*(key as *const _ as *const [u8; 8])) }
}

pub fn extract_value_type(key: &[u8]) -> u8 {
    let l = key.len();
    assert!(l >= 8);
    let num = extract_internal_key_footer(&key[(l - 8)..]);
    (num & 0xffu64) as u8
}

pub fn is_value_type(t: u8) -> bool {
    t <= ValueType::TypeMerge as u8 || t == ValueType::TypeBlobIndex as u8
}

pub fn is_extended_value_type(t: u8) -> bool {
    t <= ValueType::TypeMerge as u8
        || t == ValueType::TypeBlobIndex as u8
        || t == ValueType::TypeRangeDeletion as u8
}

pub struct GlobalSeqnoAppliedKey {
    internal_key: Vec<u8>,
    global_seqno: u64,
    is_user_key: bool,
}

impl GlobalSeqnoAppliedKey {
    pub fn new(global_seqno: u64, is_user_key: bool) -> Self {
        Self {
            internal_key: vec![],
            global_seqno,
            is_user_key,
        }
    }

    pub fn get_key(&self) -> &[u8] {
        &self.internal_key
    }

    pub fn get_user_key(&self) -> &[u8] {
        if self.is_user_key {
            &self.internal_key
        } else {
            extract_user_key(&self.internal_key)
        }
    }

    pub fn set_user_key(&mut self, key: &[u8]) {
        self.is_user_key = true;
        self.set_key(key);
    }

    pub fn set_key(&mut self, key: &[u8]) {
        if self.global_seqno == DISABLE_GLOBAL_SEQUENCE_NUMBER {
            self.internal_key.clear();
            self.internal_key.extend_from_slice(key);
            return;
        }
        let tail_offset = key.len() - 8;
        let num = decode_fixed_uint64(&key[tail_offset..]);
        self.internal_key.extend_from_slice(&key[..tail_offset]);
        let num = pack_sequence_and_type(self.global_seqno, (num & 0xff) as u8);
        self.internal_key.extend_from_slice(&num.to_le_bytes());
    }

    pub fn trim_append(&mut self, data: &[u8], offset: usize, shared: usize, non_shared: usize) {
        self.internal_key.resize(shared, 0);
        self.internal_key
            .extend_from_slice(&data[offset..(offset + non_shared)]);
    }
}
