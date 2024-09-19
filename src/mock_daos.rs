use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::os::raw::{c_char, c_int, c_uint, c_ulong};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::{
    d_iov_t, d_sg_list_t, daos_anchor_t, daos_array_iod_t, daos_cont_info_t, daos_epoch_range_t,
    daos_epoch_t, daos_event_t, daos_handle_t, daos_obj_id_t, daos_oclass_hints_t,
    daos_oclass_id_t, daos_off_t, daos_otype_t, daos_pool_info_t, daos_range_t, daos_size_t,
    daos_snapshot_opts,
};
use once_cell::sync::Lazy;

// Memory storage structure for storing OID and data
type Storage = Arc<Mutex<HashMap<daos_obj_id_t, Vec<u8>>>>;
static STORAGE: Lazy<Storage> = Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// Global counter
static HANDLE_COUNTER: AtomicU64 = AtomicU64::new(123);

pub unsafe fn daos_array_close(oh: daos_handle_t, ev: *mut daos_event_t) -> c_int {
    let mut storage = STORAGE.lock().unwrap();
    // Assuming `oh.cookie` is the `u64` part of `daos_obj_id_t`
    let obj_id = daos_obj_id_t {
        lo: oh.cookie,
        hi: 0,
    };
    println!("[mock] close {:?}", obj_id);
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
    if !storage.contains_key(&oid) {
        storage.insert(oid, Vec::new());
    }

    // Set the output parameter
    if !oh.is_null() {
        *oh = new_handle;
    }

    println!("[mock] open/create oid {:?}", oid);
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
    let data = storage.get(&obj_id).unwrap_or(&binding);
    let sgl = *sgl;
    let iovs = sgl.sg_iovs;
    let mut data_offset = 0; // To track the position in `data`

    for i in 0..sgl.sg_nr {
        let mut iov = *iovs.add(i as usize);
        // println!("[debug] iov: {:?}", iov);
        let buf = std::slice::from_raw_parts_mut(iov.iov_buf as *mut u8, iov.iov_len);
        // Copy data into `buf`
        let len_to_copy = std::cmp::min(iov.iov_len, data.len() - data_offset);
        buf[..len_to_copy].copy_from_slice(&data[data_offset..data_offset + len_to_copy]);
        data_offset += len_to_copy;
        // Update `iov_len` to the actual length written
        iov.iov_len = len_to_copy;
    }
    println!("[mock] oid {:?} -> get data {:?}", obj_id, data.get(2));
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

    if let Some(data) = storage.get_mut(&obj_id) {
        // Clear existing data
        // data.clear();

        let sgl = *sgl;
        let iovs = sgl.sg_iovs;
        let arr_rgs = (*iod).arr_rgs;
        for i in 0..sgl.sg_nr {
            let iov = *iovs.add(i as usize);
            let rg = arr_rgs.add(i as usize);
            // println!("[debug] iov: {:?}", iov);
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

        println!("[mock] oid {:?} -> new data {:?}", obj_id, data.get(2));
        0
    } else {
        println!("daos_array_write failed: handle not found");
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

pub unsafe fn daos_obj_generate_oid2(
    coh: daos_handle_t,
    oid: *mut daos_obj_id_t,
    type_: daos_otype_t,
    cid: daos_oclass_id_t,
    hints: daos_oclass_hints_t,
    args: u32,
) -> c_int {
    // Example: Generate a mock OID
    if !oid.is_null() {
        let new_handle_value = HANDLE_COUNTER.fetch_add(1, Ordering::SeqCst);
        *oid = daos_obj_id_t {
            lo: new_handle_value,
            hi: 0,
        };
        println!("[mock] new oid {:?}", *oid);
    }
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
    println!("[mock] delete oid {:?}", obj_id);
    0
}

pub unsafe fn daos_cont_create_snap(
    coh: daos_handle_t,
    epoch: *mut daos_epoch_t,
    name: *mut c_char,
    ev: *mut daos_event_t,
) -> c_int {
    println!(
        "daos_cont_create_snap called with coh: {:?}, epoch: {:?}, name: {:?}, ev: {:?}",
        coh, epoch, name, ev
    );
    // Example: Create a snapshot (no actual implementation here)
    0
}

pub unsafe fn daos_oit_close(oh: daos_handle_t, ev: *mut daos_event_t) -> c_int {
    println!("daos_oit_close called with oh: {:?}, ev: {:?}", oh, ev);
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
    println!(
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
    println!(
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

pub fn daos_cont_open2(
    poh: daos_handle_t,
    cont: *const c_char,
    flags: c_uint,
    coh: *mut daos_handle_t,
    info: *mut daos_cont_info_t,
    ev: *mut daos_event_t,
) -> c_int {
    0
}

pub fn daos_cont_destroy_snap(
    coh: daos_handle_t,
    epr: daos_epoch_range_t,
    ev: *mut daos_event_t,
) -> c_int {
    0
}

pub fn daos_cont_create_snap_opt(
    coh: daos_handle_t,
    epoch: *mut daos_epoch_t,
    name: *mut c_char,
    opts: daos_snapshot_opts,
    ev: *mut daos_event_t,
) -> c_int {
    0
}

pub fn daos_cont_close(coh: daos_handle_t, ev: *mut daos_event_t) -> c_int {
    0
}

pub fn daos_pool_disconnect(poh: daos_handle_t, ev: *mut daos_event_t) -> c_int {
    0
}

pub fn daos_fini() -> c_int {
    0
}

pub fn daos_array_destroy(oh: daos_handle_t, th: daos_handle_t, ev: *mut daos_event_t) -> c_int {
    0
}
