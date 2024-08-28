use ntapi::{
    ntldr::LDR_DATA_TABLE_ENTRY,
    ntpebteb::{PEB, TEB},
    winapi::ctypes::c_void,
};
use std::{ffi::CStr, slice};
use windows::Win32::System::{
    Diagnostics::Debug::IMAGE_NT_HEADERS64,
    Kernel::NT_TIB,
    SystemServices::{
        IMAGE_DOS_HEADER, IMAGE_DOS_SIGNATURE, IMAGE_EXPORT_DIRECTORY, IMAGE_NT_SIGNATURE,
    },
};

fn main() {
    unsafe {
        let address = get_module("ntdll.dll").expect("Error obtaining module address");
        get_proc(address);
    };
}

unsafe fn get_proc(dll_base: *mut c_void) {
    let dos_header = dll_base as *mut IMAGE_DOS_HEADER;
    if (*dos_header).e_magic != IMAGE_DOS_SIGNATURE {
        eprintln!("INVALID DOS SIGNATURE");
        return;
    }

    let nt_header = (dll_base as usize + (*dos_header).e_lfanew as usize) as *mut IMAGE_NT_HEADERS64;
    if (*nt_header).Signature != IMAGE_NT_SIGNATURE {
        eprintln!("INVALID NT SIGNATURE");
        return;
    }

    let export_directory = (dll_base as usize + (*nt_header).OptionalHeader.DataDirectory[0].VirtualAddress as usize) as *const IMAGE_EXPORT_DIRECTORY;
    let names = (dll_base as usize + (*export_directory).AddressOfNames as usize) as *const u32;
    let ordinals = (dll_base as usize + (*export_directory).AddressOfNameOrdinals as usize) as *const u16;
    let addresss = (dll_base as usize + (*export_directory).AddressOfFunctions as usize) as *const u32;

    for i in 0..(*export_directory).NumberOfNames as isize {
        let name = CStr::from_ptr((dll_base as usize + *names.offset(i) as usize) as *const i8).to_str().unwrap();
        let ordinal = *ordinals.offset(i);
        let address = (dll_base as usize + *addresss.offset(ordinal as isize) as usize) as *mut c_void;
        println!("NAME {} | ADDRESS: {:?} | ORDINAL: {}", name, address, ordinal);
    }
}

unsafe fn get_module(dll: &str) -> Result<*mut c_void, ()> {
    let peb = get_peb();
    let ldr = (*peb).Ldr;
    let mut list_entry = (*ldr).InLoadOrderModuleList.Flink as *mut LDR_DATA_TABLE_ENTRY;

    while !(*list_entry).DllBase.is_null() {
        let buffer = slice::from_raw_parts(
            (*list_entry).BaseDllName.Buffer,
            ((*list_entry).BaseDllName.Length / 2) as usize,
        );
        let dll_name = String::from_utf16(&buffer)
            .unwrap()
            .to_string()
            .to_lowercase();

        if dll == dll_name {
            return Ok((*list_entry).DllBase);
        }

        list_entry = (*list_entry).InLoadOrderLinks.Flink as *mut LDR_DATA_TABLE_ENTRY;
    }

    Err(())
}

unsafe fn get_peb() -> *mut PEB {
    let teb_offset = ntapi::FIELD_OFFSET!(NT_TIB, Self_) as u32;

    #[cfg(target_arch = "x86_64")]
    {
        use ntapi::winapi_local::um::winnt::__readgsqword;

        let teb = __readgsqword(teb_offset) as *mut TEB;
        return (*teb).ProcessEnvironmentBlock;
    }

    #[cfg(target_arch = "x86")]
    {
        use ntapi::winapi_local::um::winnt::__readfsdword;
        let teb = __readfsdword(teb_offset) as *mut TEB;
        return (*teb).ProcessEnvironmentBlock;
    }
}