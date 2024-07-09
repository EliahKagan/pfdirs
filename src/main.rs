//! Reports information from multiple sources about where Program Files folders
//! are located on a Windows system.

use std::{io, string::FromUtf16Error};
use windows::{
    core::{Error, GUID, PCWSTR, PWSTR},
    Win32::{
        Foundation::MAX_PATH,
        System::Com::CoTaskMemFree,
        UI::Shell::{
            FOLDERID_ProgramFiles, FOLDERID_ProgramFilesX64, FOLDERID_ProgramFilesX86,
            FOLDERID_UserProgramFiles, SHGetFolderPathW, SHGetKnownFolderPath, CSIDL_PROGRAM_FILES,
            CSIDL_PROGRAM_FILESX86, KF_FLAG_DEFAULT, SHGFP_TYPE_CURRENT,
        },
    },
};
use windows_sys::Win32::System::Registry::{
    KEY_QUERY_VALUE, KEY_WOW64_32KEY, KEY_WOW64_64KEY, REG_SAM_FLAGS,
};
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

macro_rules! with_names {
    ($($ident:ident),* $(,)?) => {
        [$(
            (stringify!($ident), $ident),
        )*]
    };
}

fn column_width<'a, I>(names: I) -> usize
where
    I: IntoIterator<Item = &'a str>,
{
    names
        .into_iter()
        .map(|name| name.chars().count())
        .max()
        .unwrap_or(0)
}

fn report_environment_variables() {
    let names = [
        "ProgramFiles",
        "ProgramFiles(Arm)",
        "ProgramFiles(x86)",
        "ProgramW6432",
    ];

    let width = column_width(names);

    println!("Relevant environment variables:");
    println!();

    for name in names {
        let path_item = std::env::var(name).unwrap_or_else(|e| format!("[{e}]"));
        println!("  {name:<width$}  {path_item}");
    }

    println!();
}

struct CoStr {
    pwstr: PWSTR,
}

impl CoStr {
    fn new(pwstr: PWSTR) -> Self {
        Self { pwstr }
    }

    fn to_string(&self) -> Result<String, FromUtf16Error> {
        unsafe { self.pwstr.to_string() }
    }
}

impl Drop for CoStr {
    fn drop(&mut self) {
        unsafe { CoTaskMemFree(Some(self.pwstr.as_ptr() as *const _)) };
    }
}

// FIXME: Figure out if we should also check with other flags than KF_FLAG_DEFAULT.
fn try_get_known_folder_path(id: GUID) -> Result<String, Error> {
    match unsafe { SHGetKnownFolderPath(&id, KF_FLAG_DEFAULT, None) } {
        Ok(pwstr) => Ok(CoStr::new(pwstr).to_string()?),
        Err(e) => Err(e),
    }
}

fn report_known_folders() -> Result<(), Error> {
    // TODO: If we can get the names without initializing COM, do so and display them as well.
    let folders = with_names!(
        FOLDERID_ProgramFiles,
        FOLDERID_ProgramFilesX64,
        FOLDERID_ProgramFilesX86,
        FOLDERID_UserProgramFiles,
    );

    let width = column_width(folders.map(|(name, _)| name));

    println!("Relevant known folders:");
    println!();

    for (symbol, id) in folders {
        let path_item = try_get_known_folder_path(id).unwrap_or_else(|e| format!("[{e}]"));
        println!("  {symbol:<width$}  {path_item}");
    }

    println!();
    Ok(())
}

fn try_get_path_from_csidl(csidl: u32) -> Result<String, Error> {
    let mut buffer = [0u16; MAX_PATH as usize];

    let path = unsafe {
        SHGetFolderPathW(
            None,
            csidl as i32,
            None,
            SHGFP_TYPE_CURRENT.0 as u32,
            &mut buffer,
        )?;

        PCWSTR::from_raw(buffer.as_ptr()).to_string()?
    };

    Ok(path)
}

fn report_csidl() -> Result<(), Error> {
    let folders = with_names!(
        CSIDL_PROGRAM_FILES,    // Corresponds to: FOLDERID_ProgramFiles
        CSIDL_PROGRAM_FILESX86, // Corresponds to: FOLDERID_ProgramFilesX86
    );

    let width = column_width(folders.map(|(name, _)| name));

    println!("Relevant CSIDLs:");
    println!();

    for (symbol, id) in folders {
        let path_item = try_get_path_from_csidl(id).unwrap_or_else(|e| format!("[{e}]"));
        println!("  {symbol:<width$}  {path_item}");
    }

    println!();
    Ok(())
}

fn report_registry_view(caption: &str, flag_for_view: REG_SAM_FLAGS) -> Result<(), io::Error> {
    let key_names = [
        "ProgramFilesDir",
        "ProgramFilesDir (Arm)",
        "ProgramFilesDir (x86)",
        // "ProgramFilesPath", // Less interesting, should be the literal string: %ProgramFiles%
        "ProgramW6432Dir",
    ];

    let width = column_width(key_names);

    let cur_ver = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(
        r#"SOFTWARE\Microsoft\Windows\CurrentVersion"#,
        KEY_QUERY_VALUE | flag_for_view,
    )?;

    println!("Relevant registry keys - with {caption}:");
    println!();

    for key_name in key_names {
        let path_item = cur_ver
            .get_value(key_name)
            .unwrap_or_else(|e| format!("[{e}]"));
        println!("  {key_name:<width$}  {path_item}");
    }

    println!();
    Ok(())
}

fn report_all_registry_views() -> Result<(), io::Error> {
    let views = [
        ("default view", 0),
        ("KEY_WOW64_32KEY", KEY_WOW64_32KEY),
        ("KEY_WOW64_32KEY", KEY_WOW64_64KEY),
    ];

    for (caption, flag_for_view) in views {
        report_registry_view(caption, flag_for_view)?;
    }

    Ok(())
}

fn main() -> Result<(), Error> {
    report_environment_variables();
    report_known_folders()?;
    report_csidl()?;
    report_all_registry_views()?;
    Ok(())
}
