//! Reports information from multiple sources about where program files directories are located on
//! a Windows system.
//!
//! Example output, from a 32-bit process running on an x86_64 Windows system:
//!
//! ```text
//! Relevant environment variables:
//!
//!   ProgramFiles       C:\Program Files (x86)
//!   ProgramFiles(Arm)  [environment variable not found]
//!   ProgramFiles(x86)  C:\Program Files (x86)
//!   ProgramW6432       C:\Program Files
//!
//! Relevant known folders:
//!
//!   FOLDERID_ProgramFiles      C:\Program Files (x86)
//!   FOLDERID_ProgramFilesX64   [The system cannot find the file specified. (0x80070002)]
//!   FOLDERID_ProgramFilesX86   C:\Program Files (x86)
//!   FOLDERID_UserProgramFiles  C:\Users\ek\AppData\Local\Programs
//!
//! Relevant CSIDLs:
//!
//!   CSIDL_PROGRAM_FILES     C:\Program Files (x86)
//!   CSIDL_PROGRAM_FILESX86  C:\Program Files (x86)
//!
//! Relevant registry keys - with default view:
//!
//!   ProgramFilesDir        C:\Program Files (x86)
//!   ProgramFilesDir (Arm)  [The system cannot find the file specified. (os error 2)]
//!   ProgramFilesDir (x86)  C:\Program Files (x86)
//!   ProgramW6432Dir        C:\Program Files
//!
//! Relevant registry keys - with KEY_WOW64_32KEY:
//!
//!   ProgramFilesDir        C:\Program Files (x86)
//!   ProgramFilesDir (Arm)  [The system cannot find the file specified. (os error 2)]
//!   ProgramFilesDir (x86)  C:\Program Files (x86)
//!   ProgramW6432Dir        C:\Program Files
//!
//! Relevant registry keys - with KEY_WOW64_64KEY:
//!
//!   ProgramFilesDir        C:\Program Files
//!   ProgramFilesDir (Arm)  [The system cannot find the file specified. (os error 2)]
//!   ProgramFilesDir (x86)  C:\Program Files (x86)
//!   ProgramW6432Dir        C:\Program Files
//! ```
//!
//! On 64-bit Windows, the `ProgramFiles` environment variable, `FOLDERID_ProgramFiles` known
//! folder, `CSIDL_PROGRAM_FILES`, and `ProgramFilesDir` registry key, look up a path that differs
//! depending on whether the program accessing the information is 64-bit or 32-bit.
//!
//! On such a system, whether x86_64 (AMD64) or ARM64, a 64-bit process reports the 64-bit program
//! files directory, most often `C:\Program Files`, while a 32-bit process reports the 32-bit
//! program files directory, most often `C:\Program Files (x86)`.
//!
//! In contrast, *when available*:
//!
//! - The `ProgramFiles(x86)` environment variable, `FOLDERID_ProgramFilesX86` known folder,
//!   `CSIDL_PROGRAM_FILESX86`, and `ProgramFilesDir (x86)` registry key report the 32-bit program
//!   files directory.
//!
//! - The `ProgramW6432` environment variable, `FOLDERID_ProgramFilesX64` known folder, and
//!   `ProgramW6432Dir` registry key report the 64-bit program files directory.
//!
//! However, not all of them are always available to all processes on all Windows system.
//!
//! As detailed in comments below on specific `report_*` functions below, Microsoft documentation
//! tends to recommend obtaining such paths through the *known folders* facilities. However, as
//! shown above, even on a 64-bit system, a 32-bit process unfortunately does not see any
//! `FOLDERID_ProgramFilesX64` known folder (and there is no CSIDL corresponding to that).
//!
//! On such a system it may therefore be necessary to use either the `ProgramW6432` environment
//! variable or the `ProgramW6432Dir` registry key to get the path of the 64-bit program files
//! directory:
//!
//! - Accessing the environment variable is easy and seems to be more common. Some forms of unusual
//!   customization by a parent process of its child processes' environments will break this. See
//!   `report_environment_variables()` below for details.
//!
//! - The `ProgramW6432Dir` key appears to be available on 64-bit systems through any registry
//!   view.
//!
//! On a 32-bit system, there is no way to get the 64-bit program files directory, because there is
//! no such directory.

use core::ffi::c_void;
use std::io;
use std::string::FromUtf16Error;

use known_folders::{get_known_folder_path, KnownFolder};
use windows::core::{Error, GUID, PCWSTR, PWSTR};
use windows::Win32::Foundation::MAX_PATH;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{
    FOLDERID_ProgramFiles, FOLDERID_ProgramFilesX64, FOLDERID_ProgramFilesX86,
    FOLDERID_UserProgramFiles, SHGetFolderPathW, SHGetKnownFolderPath, CSIDL_PROGRAM_FILES,
    CSIDL_PROGRAM_FILESX86, KF_FLAG_DEFAULT, SHGFP_TYPE_CURRENT,
};
use winreg::{
    enums::{HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, KEY_WOW64_32KEY, KEY_WOW64_64KEY},
    RegKey,
};

/// Finds the width of the symbolic name column for the table of reported results.
///
/// This estimate is highly likely to be accurate, since the names in this column are known in
/// advance. (They even happen to be all ASCII characters, though this does not rely on that.)
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

/// Report *program files* folder locations contained in environment variables.
///
/// FIXME: Write the rest of this documentation comment!!
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

/// Owner of a `PWSTR` that must be freed with `CoTaskMemFree`.
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

// TODO: Figure out whether to implement windows::core::Owned instead.
impl Drop for CoStr {
    fn drop(&mut self) {
        unsafe { CoTaskMemFree(Some(self.pwstr.as_ptr().cast::<c_void>())) };
    }
}

/// Helper that calls `ShGetKnownFolderPath` on behalf of `report_known_folders()`.
///
/// TODO: Figure out if we should also check with other flags than KF_FLAG_DEFAULT.
fn get_known_folder_path_or_detailed_error(id: GUID) -> Result<String, Error> {
    match unsafe { SHGetKnownFolderPath(&id, KF_FLAG_DEFAULT, None) } {
        Ok(pwstr) => Ok(CoStr::new(pwstr).to_string()?),
        Err(e) => Err(e),
    }
}

/// Report *program files* folder locations by querying *known folders*.
///
/// This is a recommended approach. This can be done through the Windows API or indirectly through
/// a crate that wraps it. This function showcases both and asserts that the information provided,
/// where overlapping, is identical.
///
/// #### Windows API
///
/// Windows provides two approaches in its API for accessing the paths of known folders:
///
/// - The
///   [`SHGetKnownFolderPath`](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shgetknownfolderpath)
///   function. This approach is more straightforward and typically sufficient when the GUIDs are
///   known and only paths are needed. (There are a small number of other related functions for
///   obtaining other information.) This is the approach used here.
///
/// - The
///   [`IKnownFolder::GetPath`](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-iknownfolder-getpath)
///   method. This is more involved, but `IKnownFolder` COM objects are a richer source of
///   information. For example, `IKnownFolder` supports iterating over all known folders.
///
/// #### known-folders crate
///
/// The [known-folders](https://crates.io/crates/known-folders) crate provides a
/// `get_known_folder_path()` function that takes care of calling `SHGetKnownFolderPath` from Rust
/// code. However, this is limited to simple uses:
///
/// - It does not accept custom `KNOWN_FOLDER_FLAGS` or a custom access token.
///
/// - It returns an `Option` rather than a `Result`, so when a known folder path is unavailable,
///   the different errors that can cause this are not distinguished.
///
/// But in the most common cases `get_known_folder_path()` is sufficient.
///
/// #### What this function does
///
/// This uses both `SHGetKnownFolderPath`, called through the `windows` crate, and
/// `get_known_folder_path()`, provided by the `known-folders` crate, and compares the results for
/// whether there was an error and, if not, whether the paths match. Calling both is for
/// experimentation and demonstration purposes. Generally at most one of these two approaches
/// should be used, depending on requirements.
///
/// This looks up only the four folder IDs for *program files* folders. Their GUIDs are available
/// as symbolic constants both in the `windows` crate as `GUID` objects and, as a higher level
/// abstraction, in the `KnownFolder` enum of the `known-folders` crate.
fn report_known_folders() -> Result<(), Error> {
    // TODO: If we can get the names without initializing COM, do so and display them as well.
    let folders = [
        (
            "FOLDERID_ProgramFiles",
            FOLDERID_ProgramFiles,
            KnownFolder::ProgramFiles,
        ),
        (
            "FOLDERID_ProgramFilesX64",
            FOLDERID_ProgramFilesX64,
            KnownFolder::ProgramFilesX64,
        ),
        (
            "FOLDERID_ProgramFilesX86",
            FOLDERID_ProgramFilesX86,
            KnownFolder::ProgramFilesX86,
        ),
        (
            "FOLDERID_UserProgramFiles",
            FOLDERID_UserProgramFiles,
            KnownFolder::UserProgramFiles,
        ),
    ];
    let width = column_width(folders.map(|(name, _, _)| name));

    println!("Relevant known folders:");
    println!();

    for (symbol, id, kf) in folders {
        // Calling SHGetKnownFolderPath ourselves gives more detailed error information.
        let path_or_error = get_known_folder_path_or_detailed_error(id);

        // The `known-folders` crate is simple and easy to use, but gives `Option`, not `Result`.
        let maybe_path = get_known_folder_path(kf).and_then(|p| p.to_str().map(String::from));

        // Compare the information from both approaches. If inconsistent, panic with the details.
        let path_item = match (path_or_error, maybe_path) {
            (Ok(my_kf_path), Some(lib_kf_path)) if my_kf_path == lib_kf_path => my_kf_path,
            (Err(e), None) => format!("[{e}]"),
            (my_thing, lib_thing) => {
                panic!("Mismatch! We got {my_thing:?}, known_folders library got {lib_thing:?}")
            }
        };

        // Report the path obtained, or detailed error info from our own SHGetKnownFolderPath call.
        println!("  {symbol:<width$}  {path_item}");
    }

    println!();
    Ok(())
}

/// Helper that calls `SHGetFolderPathW()` on behalf of `report_csidl()`.
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

/// Report *program files* folder locations via lookups using CSIDLs.
///
/// This calls the deprecated
/// [`SHGetFolderPathW`](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shgetfolderpathw)
/// function.
///
/// This is the older way, before the *known folders* facilities were introduced. See
/// [CSIDL](https://learn.microsoft.com/en-us/windows/win32/shell/csidl).
///
/// As noted there, it is recommended to use the known folders APIs instead of CSIDLs, and each
/// CSIDL value has a corresponding `KNOWNFOLDERID` value. In contrast, not all known folders have
/// a CSIDL, and also, unlike with CSIDLs, it is possible to register new known folders
/// programmatically.
///
/// From the [remarks section](https://learn.microsoft.com/en-us/windows/win32/shell/csidl#remarks)
/// of that article:
///
/// > These values supersede the use of environment variables for this purpose. They are in turn
/// > superseded in Windows Vista and later by the
/// > [KNOWNFOLDERID](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid) values.
///
/// (This seems to imply, by transitivity, that getting the paths of known folders is also
/// preferable to accessing the values of environment variables, when both are applicable.)
///
/// One limitation of using CSIDLs is that it cannot properly handle the unusual case that the path
/// is a `\\?\` long path and exceeds
/// [`MAX_PATH`](https://learn.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation)
/// characters. As
/// [commented](https://github.com/dotnet/runtime/blob/v8.0.7/src/libraries/System.Private.CoreLib/src/System/Environment.Win32.cs#L210-L211)
/// in the implementation of the .NET Runtime:
///
/// > We're using SHGetKnownFolderPath instead of SHGetFolderPath as SHGetFolderPath is capped at MAX_PATH.
fn report_csidl() -> Result<(), Error> {
    let folders = [
        ("CSIDL_PROGRAM_FILES", CSIDL_PROGRAM_FILES), // Corresponds to: FOLDERID_ProgramFiles
        ("CSIDL_PROGRAM_FILESX86", CSIDL_PROGRAM_FILESX86), // Corresponds to: FOLDERID_ProgramFilesX86
    ];
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

/// Report *program files* folder locations from a single specified view of the registry.
///
/// See `report_all_registry_views()` for more information on views.
///
/// This accesses subkeys of `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion` using the `winreg`
/// crate, which uses
/// [`RegOpenKeyExW`](https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regopenkeyexw).
fn report_registry_view(caption: &str, flag_for_view: u32) -> Result<(), io::Error> {
    let key_names = [
        "ProgramFilesDir",
        "ProgramFilesDir (Arm)",
        "ProgramFilesDir (x86)",
        // "ProgramFilesPath", // Less interesting, usually literal %ProgramFiles% if got this way.
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

/// Report *program files* folder locations from multiple views of the registry.
///
/// See also:
///
/// - [Accessing an Alternate Registry View](https://learn.microsoft.com/en-us/windows/win32/winprog64/accessing-an-alternate-registry-view)
///   for details on registry views that can be accessed.
///
/// - `report_registry_view()` for details on how the lookup is performed.
fn report_all_registry_views() -> Result<(), io::Error> {
    let views = [
        ("default view", 0),
        ("KEY_WOW64_32KEY", KEY_WOW64_32KEY),
        ("KEY_WOW64_64KEY", KEY_WOW64_64KEY),
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
