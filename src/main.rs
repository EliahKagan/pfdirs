//! Reports information from multiple sources about where `Program Files`
//! folders are located on a Windows system.

use std::string::FromUtf16Error;

use windows::{
    core::{Error, GUID, PWSTR},
    Win32::System::Com::CoTaskMemFree,
    Win32::UI::Shell::{
        FOLDERID_ProgramFiles, FOLDERID_ProgramFilesX64, FOLDERID_ProgramFilesX86,
        FOLDERID_UserProgramFiles, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
    },
};

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
    // TODO: Avoid hard-coding the name strings, unless that forces us to initialize COM.
    let folders = [
        ("ProgramFiles", FOLDERID_ProgramFiles),
        ("ProgramFilesX64", FOLDERID_ProgramFilesX64),
        ("ProgramFilesX86", FOLDERID_ProgramFilesX86),
        ("UserProgramFiles", FOLDERID_UserProgramFiles),
    ];

    let width = column_width(folders.map(|(name, _)| name));

    println!("Relevant known folders:");
    println!();

    for (name, id) in folders {
        let path_item =
            try_get_known_folder_path(id).unwrap_or_else(|e| format!("[{}]", e.message()));
        println!("  {name:<width$}  {path_item}");
    }

    Ok(())
}

fn main() -> Result<(), Error> {
    report_environment_variables();
    println!();
    report_known_folders()?;

    // FIXME: Do the other reports.

    Ok(())
}
