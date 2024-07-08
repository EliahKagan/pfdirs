//! Reports information from multiple sources about where `Program Files`
//! folders are located on a Windows system.

use std::string::FromUtf16Error;

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
        let path_item = std::env::var(name).unwrap_or_else(|_| "[variable does not exist]".into());
        println!("  {name:<width$}  {path_item}");
    }
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
    // TODO: If we don't have to initialize COM to get the names, do that too (3 columns).
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
        let path_item =
            try_get_known_folder_path(id).unwrap_or_else(|e| format!("[{}]", e.message()));
        println!("  {symbol:<width$}  {path_item}");
    }

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
        CSIDL_PROGRAM_FILES, // FOLDERID_ProgramFiles
        CSIDL_PROGRAM_FILESX86, // FOLDERID_ProgramFilesX86
    );

    let width = column_width(folders.map(|(name, _)| name));

    println!("Relevant CSIDLs:");
    println!();

    for (symbol, id) in folders {
        let path_item =
            try_get_path_from_csidl(id).unwrap_or_else(|e| format!("[{}]", e.message()));
        println!("  {symbol:<width$}  {path_item}");
    }

    Ok(())
}

fn main() -> Result<(), Error> {
    report_environment_variables();
    println!();
    report_known_folders()?;
    println!();
    report_csidl()?;

    // FIXME: Do the other reports, at least the registry.

    Ok(())
}
