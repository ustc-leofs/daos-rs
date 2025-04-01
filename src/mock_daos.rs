use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::os::raw::{c_char, c_int, c_uint, c_ulong};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use log::info;

use crate::{
    d_iov_t, d_rank_list_t, d_sg_list_t, daos_anchor_t, daos_array_iod_t, daos_cont_info_t,
    daos_epoch_range_t, daos_epoch_t, daos_event, daos_event_t, daos_handle_t, daos_obj_id_t,
    daos_oclass_hints_t, daos_oclass_id_t, daos_off_t, daos_otype_t, daos_pool_info_t, daos_prop_t,
    daos_range_t, daos_size_t, daos_snapshot_opts,
};
use once_cell::sync::Lazy;
use std::cmp::PartialEq;

pub const DAOS_API_VERSION_MINOR: u32 = 999; // 覆盖 bindings.rs 中的同名常量

impl Hash for daos_obj_id_t {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // 使用结构体的字段进行哈希计算
        self.lo.hash(state);
        self.hi.hash(state);
    }
}

// 手动实现 PartialEq
impl PartialEq for daos_obj_id_t {
    fn eq(&self, other: &Self) -> bool {
        // 自定义的比较逻辑：这里我们简单比较 lo 和 hi 字段是否相等
        self.lo == other.lo && self.hi == other.hi
    }
}

// 实现 Eq（只要 PartialEq 实现了，Eq 是空的，不需要额外逻辑）
impl Eq for daos_obj_id_t {}

// Memory storage structure for storing OID and data

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::FileExt;
use std::path::Path;

static HANDLE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct Storage {
    path: String,
    file_lock: Mutex<()>,
}

impl Storage {
    pub fn new() -> Storage {
        let tmp_path = Path::new(std::env::temp_dir().as_path()).join("daos_mock_storage");
        let storage_path = tmp_path.display().to_string();
        info!("mock storage_path: {:?}", &storage_path);
        fs::create_dir_all(&storage_path).expect("Failed to create storage directory");

        let max_id = fs::read_dir(&storage_path)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter_map(|name| {
                info!("name: {:?}", name);
                name.parse::<u64>().ok()
            })
            .max()
            .unwrap_or(0);

        HANDLE_COUNTER.store(max_id + 1, Ordering::SeqCst);

        Storage {
            path: storage_path,
            file_lock: Mutex::new(()),
        }
    }

    pub fn get(&self, oid: &daos_obj_id_t) -> Option<Vec<u8>> {
        let file_path = format!("{}/{}", self.path, oid.lo);
        let file = File::open(&file_path).ok()?;
        let mut buffer = Vec::new();
        file.take(u64::MAX).read_to_end(&mut buffer).ok()?;
        Some(buffer)
    }

    pub fn contains(&self, oid: &daos_obj_id_t) -> bool {
        let file_path = format!("{}/{}", self.path, oid.lo);
        Path::new(&file_path).exists()
    }

    pub fn insert(&self, oid: daos_obj_id_t, data: Vec<u8>) {
        let _lock = self.file_lock.lock().unwrap();
        let file_path = format!("{}/{}", self.path, oid.lo);
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&file_path)
            .unwrap();
        file.write_all(&data).unwrap();
    }

    pub fn remove(&self, oid: &daos_obj_id_t) {
        let _lock = self.file_lock.lock().unwrap();
        let file_path = format!("{}/{}", self.path, oid.lo);
        let _ = fs::remove_file(file_path);
    }
}

static STORAGE: Lazy<Arc<Mutex<Storage>>> = Lazy::new(|| Arc::new(Mutex::new(Storage::new())));

pub unsafe fn daos_array_close(oh: daos_handle_t, ev: *mut daos_event_t) -> ::std::os::raw::c_int {
    let mut storage = STORAGE.lock().unwrap();
    // Assuming `oh.cookie` is the `u64` part of `daos_obj_id_t`
    let obj_id = daos_obj_id_t {
        lo: oh.cookie,
        hi: 0,
    };
    info!("[mock] close {:?}", obj_id);
    0
}

pub unsafe fn daos_array_open_with_attr(
    coh: daos_handle_t,
    oid: daos_obj_id_t,
    th: daos_handle_t,
    mode: c_uint,
    cell_size: daos_size_t,
    chunk_size: daos_size_t,
    oh: *mut daos_handle_t,
    ev: *mut daos_event_t,
) -> c_int {
    let mut storage = STORAGE.lock().unwrap();

    // Generate a new handle
    let new_handle = daos_handle_t { cookie: oid.lo };

    // Store the OID in the storage
    if !storage.contains(&oid) {
        storage.insert(oid, Vec::new());
    }

    // Set the output parameter
    if !oh.is_null() {
        *oh = new_handle;
    }

    info!("[mock] open/create oid {:?}", oid);
    0
}

pub unsafe fn daos_array_read(
    oh: daos_handle_t,
    th: daos_handle_t,
    iod: *mut daos_array_iod_t,
    sgl: *mut d_sg_list_t,
    ev: *mut daos_event_t,
) -> c_int {
    let storage = STORAGE.lock().unwrap();
    // Assuming `oh.cookie` is the `u64` part of `daos_obj_id_t`
    let obj_id = daos_obj_id_t {
        lo: oh.cookie,
        hi: 0,
    };
    let binding = Vec::new();
    let data = storage.get(&obj_id).unwrap_or(binding);
    let sgl = *sgl;
    let iovs = sgl.sg_iovs;
    let mut data_offset = 0; // To track the position in `data`

    for i in 0..sgl.sg_nr {
        let mut iov = *iovs.add(i as usize);
        // debug!("iov: {:?}", iov);
        let buf = std::slice::from_raw_parts_mut(iov.iov_buf as *mut u8, iov.iov_len);
        // Copy data into `buf`
        let len_to_copy = std::cmp::min(iov.iov_len, data.len() - data_offset);
        buf[..len_to_copy].copy_from_slice(&data[data_offset..data_offset + len_to_copy]);
        (*iod).arr_nr_read += len_to_copy as u64;
        data_offset += len_to_copy;
        // Update `iov_len` to the actual length written
        iov.iov_len = len_to_copy;
    }
    info!("[mock] oid {:?} -> get data {:?}", obj_id, data.get(2));
    0
}

pub unsafe fn daos_array_write(
    oh: daos_handle_t,
    th: daos_handle_t,
    iod: *mut daos_array_iod_t,
    sgl: *mut d_sg_list_t,
    ev: *mut daos_event_t,
) -> c_int {
    let mut storage = STORAGE.lock().unwrap();
    // Assuming `oh.cookie` is the `u64` part of `daos_obj_id_t`
    let obj_id = daos_obj_id_t {
        lo: oh.cookie,
        hi: 0,
    };

    if let Some(data) = storage.get(&obj_id) {
        // Clear existing data
        // data.clear();
        let mut data = data.clone();

        let sgl = *sgl;
        let iovs = sgl.sg_iovs;
        let arr_rgs = (*iod).arr_rgs;
        for i in 0..sgl.sg_nr {
            let iov = *iovs.add(i as usize);
            let rg = arr_rgs.add(i as usize);
            // debug!("iov: {:?}", iov);
            let buf = std::slice::from_raw_parts(iov.iov_buf as *const u8, iov.iov_len);
            let offset = (*rg).rg_idx as usize;
            let array_size = (*rg).rg_len as usize;
            let len = array_size + offset;
            if data.len() < len {
                data.resize(len, 0);
            }
            let (left, right) = data.split_at_mut(offset);
            right[0..buf.len()].copy_from_slice(buf);
        }
        info!("[mock] oid {:?} -> new data {:?}", obj_id, data.get(2));
        storage.insert(obj_id, data);

        0
    } else {
        info!("daos_array_write failed: handle not found");
        -1
    }
}

pub unsafe fn daos_cont_alloc_oids(
    coh: daos_handle_t,
    num_oids: daos_size_t,
    oid: *mut u64,
    ev: *mut daos_event_t,
) -> c_int {
    0
}

pub unsafe fn daos_cont_query(
    coh: daos_handle_t,
    info: *mut daos_cont_info_t,
    cont_prop: *mut daos_prop_t,
    ev: *mut daos_event_t,
) -> ::std::os::raw::c_int {
    0
}

pub unsafe fn daos_pool_query(
    poh: daos_handle_t,
    ranks: *mut *mut d_rank_list_t,
    info: *mut daos_pool_info_t,
    pool_prop: *mut daos_prop_t,
    ev: *mut daos_event_t,
) -> ::std::os::raw::c_int {
    0
}

pub unsafe fn daos_obj_generate_oid2(
    coh: daos_handle_t,
    oid: *mut daos_obj_id_t,
    type_: daos_otype_t,
    cid: daos_oclass_id_t,
    hints: daos_oclass_hints_t,
    args: u32,
) -> c_int {
    // Example: Generate a mock OID
    let _store = STORAGE.lock().unwrap();
    if !oid.is_null() {
        let new_handle_value = HANDLE_COUNTER.fetch_add(1, Ordering::SeqCst);
        *oid = daos_obj_id_t {
            lo: new_handle_value,
            hi: 0,
        };
        info!("[mock] new oid {:?}", *oid);
    }
    0
}

pub unsafe fn daos_array_punch(
    oh: daos_handle_t,
    th: daos_handle_t,
    iod: *mut daos_array_iod_t,
    ev: *mut daos_event_t,
) -> ::std::os::raw::c_int {
    0
}

pub unsafe fn daos_obj_punch(
    oh: daos_handle_t,
    th: daos_handle_t,
    flags: u64,
    ev: *mut daos_event_t,
) -> c_int {
    let mut storage = STORAGE.lock().unwrap();
    let obj_id = daos_obj_id_t {
        lo: oh.cookie,
        hi: 0,
    };
    storage.remove(&obj_id);
    info!("[mock] delete oid {:?}", obj_id);
    0
}

pub unsafe fn daos_cont_create_snap(
    coh: daos_handle_t,
    epoch: *mut daos_epoch_t,
    name: *mut c_char,
    ev: *mut daos_event_t,
) -> c_int {
    info!(
        "daos_cont_create_snap called with coh: {:?}, epoch: {:?}, name: {:?}, ev: {:?}",
        coh, epoch, name, ev
    );
    // Example: Create a snapshot (no actual implementation here)
    0
}

pub unsafe fn daos_oit_close(oh: daos_handle_t, ev: *mut daos_event_t) -> c_int {
    info!("daos_oit_close called with oh: {:?}, ev: {:?}", oh, ev);
    // Example: Close OIT (may require additional operations in actual implementation)
    0
}

pub unsafe fn daos_oit_list(
    oh: daos_handle_t,
    oids: *mut daos_obj_id_t,
    oids_nr: *mut u32,
    anchor: *mut daos_anchor_t,
    ev: *mut daos_event_t,
) -> c_int {
    info!(
        "daos_oit_list called with oh: {:?}, oids: {:?}, oids_nr: {:?}, anchor: {:?}, ev: {:?}",
        oh, oids, oids_nr, anchor, ev
    );
    // Example: List OIT (may require additional operations in actual implementation)
    0
}

pub unsafe fn daos_oit_open(
    coh: daos_handle_t,
    epoch: daos_epoch_t,
    oh: *mut daos_handle_t,
    ev: *mut daos_event_t,
) -> c_int {
    info!(
        "daos_oit_open called with coh: {:?}, epoch: {}, oh: {:?}, ev: {:?}",
        coh, epoch, oh, ev
    );
    // Example: Open OIT (may require additional operations in actual implementation)
    0
}

pub unsafe fn daos_init() -> c_int {
    0
}

pub unsafe fn daos_pool_connect2(
    pool: *const c_char,
    sys: *const c_char,
    flags: c_uint,
    poh: *mut daos_handle_t,
    info: *mut daos_pool_info_t,
    ev: *mut daos_event_t,
) -> c_int {
    0
}

pub unsafe fn daos_cont_open2(
    poh: daos_handle_t,
    cont: *const c_char,
    flags: c_uint,
    coh: *mut daos_handle_t,
    info: *mut daos_cont_info_t,
    ev: *mut daos_event_t,
) -> c_int {
    0
}

pub unsafe fn daos_cont_destroy_snap(
    coh: daos_handle_t,
    epr: daos_epoch_range_t,
    ev: *mut daos_event_t,
) -> c_int {
    0
}

pub unsafe fn daos_cont_create_snap_opt(
    coh: daos_handle_t,
    epoch: *mut daos_epoch_t,
    name: *mut c_char,
    opts: daos_snapshot_opts,
    ev: *mut daos_event_t,
) -> c_int {
    0
}

pub unsafe fn daos_cont_close(coh: daos_handle_t, ev: *mut daos_event_t) -> c_int {
    0
}

pub unsafe fn daos_pool_disconnect(poh: daos_handle_t, ev: *mut daos_event_t) -> c_int {
    0
}

pub unsafe fn daos_fini() -> c_int {
    0
}

pub unsafe fn daos_array_destroy(
    oh: daos_handle_t,
    th: daos_handle_t,
    ev: *mut daos_event_t,
) -> c_int {
    let mut storage = STORAGE.lock().unwrap();
    let obj_id = daos_obj_id_t {
        lo: oh.cookie,
        hi: 0,
    };
    storage.remove(&obj_id);
    info!("[mock] delete oid {:?}", obj_id);
    0
}

pub unsafe fn daos_array_set_size(
    oh: daos_handle_t,
    th: daos_handle_t,
    size: daos_size_t,
    ev: *mut daos_event_t,
) -> c_int {
    let mut storage = STORAGE.lock().unwrap();
    // Assuming `oh.cookie` is the `u64` part of `daos_obj_id_t`
    let obj_id = daos_obj_id_t {
        lo: oh.cookie,
        hi: 0,
    };

    if let Some(data) = storage.get(&obj_id) {
        let mut data = data.clone();
        data.resize(size as usize, 0);
        storage.insert(obj_id, data.clone());
    }
    0
}

pub unsafe fn daos_array_get_size(
    oh: daos_handle_t,
    th: daos_handle_t,
    size: *mut daos_size_t,
    ev: *mut daos_event_t,
) -> c_int {
    let mut storage = STORAGE.lock().unwrap();
    // Assuming `oh.cookie` is the `u64` part of `daos_obj_id_t`
    let obj_id = daos_obj_id_t {
        lo: oh.cookie,
        hi: 0,
    };

    if let Some(data) = storage.get(&obj_id) {
        unsafe { *size = data.len() as u64 };
    }
    0
}

pub unsafe fn daos_eq_lib_reset_after_fork() -> ::std::os::raw::c_int {
    0
}

pub unsafe fn daos_eq_create(eqh: *mut daos_handle_t) -> ::std::os::raw::c_int {
    0
}

pub unsafe fn daos_eq_destroy(
    eqh: daos_handle_t,
    flags: ::std::os::raw::c_int,
) -> ::std::os::raw::c_int {
    0
}

pub unsafe fn daos_event_fini(ev: *mut daos_event_t) -> ::std::os::raw::c_int {
    0
}

pub unsafe fn daos_event_test(
    ev: *mut daos_event,
    timeout: i64,
    flag: *mut bool,
) -> ::std::os::raw::c_int {
    *flag = true;
    0
}

pub unsafe fn daos_event_init(
    ev: *mut daos_event_t,
    eqh: daos_handle_t,
    parent: *mut daos_event_t,
) -> ::std::os::raw::c_int {
    0
}

pub unsafe fn mock_test() {
    println!("mock_test");
}
