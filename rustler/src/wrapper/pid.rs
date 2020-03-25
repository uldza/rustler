use crate::wrapper::{ErlNifPid, NIF_ENV, NIF_TERM};
use std::mem::MaybeUninit;

pub unsafe fn get_local_pid(env: NIF_ENV, term: NIF_TERM) -> Option<ErlNifPid> {
    let mut pid = MaybeUninit::uninit();
    if rustler_sys::enif_get_local_pid(env, term, pid.as_mut_ptr()) == 0 {
        return None;
    }
    Some(pid.assume_init())
}

// pub unsafe fn is_process_alive(env: NIF_ENV, pid: &ErlNifPid) -> bool {
//     rustler_sys::enif_is_process_alive(env, pid) != 0
// }

pub unsafe fn make_pid(env: NIF_ENV, pid: ErlNifPid) -> NIF_TERM {
    rustler_sys::enif_make_pid(env, pid)
}
