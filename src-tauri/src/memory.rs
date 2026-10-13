use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

#[link(name = "ntdll")]
extern "system" {
    fn NtSetSystemInformation(class: i32, info: *mut core::ffi::c_void, len: u32) -> i32;
}

const SYSTEM_MEMORY_LIST_INFORMATION: i32 = 80;
const MEMORY_FLUSH_MODIFIED_LIST: i32 = 3;
const MEMORY_PURGE_STANDBY_LIST: i32 = 4;

pub struct Status {
    pub avail: u64,
    pub load: u32,
}

pub fn status() -> Status {
    unsafe {
        let mut m: MEMORYSTATUSEX = std::mem::zeroed();
        m.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        GlobalMemoryStatusEx(&mut m);
        Status { avail: m.ullAvailPhys, load: m.dwMemoryLoad }
    }
}

fn enable_privilege(name: &str) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, LUID};
    use windows_sys::Win32::Security::{
        AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
        TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut luid: LUID = std::mem::zeroed();
        let ok = LookupPrivilegeValueW(std::ptr::null(), wide.as_ptr(), &mut luid) != 0 && {
            let tp = TOKEN_PRIVILEGES {
                PrivilegeCount: 1,
                Privileges: [LUID_AND_ATTRIBUTES { Luid: luid, Attributes: SE_PRIVILEGE_ENABLED }],
            };
            AdjustTokenPrivileges(token, 0, &tp, 0, std::ptr::null_mut(), std::ptr::null_mut()) != 0
        };
        CloseHandle(token);
        ok
    }
}

pub fn clean() -> i64 {
    let before = status().avail as i64;
    enable_privilege("SeProfileSingleProcessPrivilege");
    enable_privilege("SeIncreaseQuotaPrivilege");
    unsafe {
        for cmd in [MEMORY_FLUSH_MODIFIED_LIST, MEMORY_PURGE_STANDBY_LIST] {
            let mut c = cmd;
            NtSetSystemInformation(
                SYSTEM_MEMORY_LIST_INFORMATION,
                &mut c as *mut i32 as *mut core::ffi::c_void,
                4,
            );
        }
    }
    (status().avail as i64 - before) / (1024 * 1024)
}
