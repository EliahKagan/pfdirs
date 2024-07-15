# Find "Program Files" folders in several ways

This is Rust program that reports information from multiple sources about where program files directories are located on a Windows system.

## License

[0BSD](LICENSE)

## Sources of information

Details, including on subtleties of availability across architectures, as provided [in the code](`src/main.rs`) on the four `report_*` functions, which are only summarized here:

- **`report_environment_variables()`** uses the `ProgramFiles`, `ProgramFilesW6432`, `ProgramFiles(x86)`, and `ProgramFiles(ARM)` [*environment variables*](https://learn.microsoft.com/en-us/windows/win32/winprog64/wow64-implementation-details#environment-variables).

  It calls [`std::env::var()`](https://doc.rust-lang.org/std/env/fn.var.html) which, on Windows, [itself](https://github.com/rust-lang/rust/blob/1.79.0/library/std/src/env.rs#L205-L272) internally [calls](https://github.com/rust-lang/rust/blob/129f3b9964af4d4a709d1383930ade12dfe7c081/library/std/src/sys/pal/windows/os.rs#L296-L303) the [`GetEnvironmentVariableW`](https://learn.microsoft.com/en-us/windows/win32/api/processenv/nf-processenv-getenvironmentvariablew) function.

- **`report_known_folders()`** uses the [`ProgramFiles`](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid#FOLDERID_ProgramFiles), [`ProgramFilesX64`](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid#FOLDERID_ProgramFilesX64), [`ProgramFilesX86`](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid#FOLDERID_ProgramFilesX86), and [`UserProgramFiles`](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid#FOLDERID_UserProgramFiles) [*known folders*](https://learn.microsoft.com/en-us/windows/win32/shell/known-folders).

  It calls [`SHGetKnownFolderPath`](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shgetknownfolderpath) in the Windows API using the [`windows`](https://crates.io/crates/known-folders) crate, which allows detailed errors to be reported, and for demonstration purposes also calls and checks those results against the [`get_known_folder_path()`](https://docs.rs/known-folders/1.1.0/known_folders/fn.get_known_folder_path.html) function provided by the [`known-folders`](https://crates.io/crates/known-folders) crate, which is often sufficient.

- **`report_csidl()`** uses the [`CSIDL_PROGRAM_FILES`](https://learn.microsoft.com/en-us/windows/win32/shell/csidl#CSIDL_PROGRAM_FILES) and [`CSIDL_PROGRAM_FILESX86`](https://learn.microsoft.com/en-us/windows/win32/shell/csidl#CSIDL_PROGRAM_FILESX86) [*CSIDLs*](https://learn.microsoft.com/en-us/windows/win32/shell/csidl), though this should not usually be done because CSIDLs are [superseded](https://learn.microsoft.com/en-us/windows/win32/shell/csidl#remarks) by known folders.

  It calls [`SHGetFolderPathW`](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shgetfolderpathw) in the Windows API using the [`windows`](https://crates.io/crates/known-folders) crate.

- **`report_all_registry_views()`** (see also **`report_registry_view()`**) uses the `ProgramFilesDir`, `ProgramW6432Dir`, `ProgramFilesDir (x86)`, and `ProgramFilesDir (Arm)` *registry keys* in `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion`, showing their values when accessed through the default view of the registry that depends on the process architecture, as well as when [explicitly specifying](https://learn.microsoft.com/en-us/windows/win32/winprog64/accessing-an-alternate-registry-view) the 32-bit view with `KEY_WOW64_32KEY` or the 64-bit view with `KEY_WOW64_64KEY`.

  It calls [`RegKey::open_subkey_with_flags`](https://docs.rs/winreg/0.52.0/winreg/reg_key/struct.RegKey.html#method.open_subkey_with_flags) in the [`winreg`](https://crates.io/crates/winreg) crate, which [itself calls](https://docs.rs/winreg/0.52.0/src/winreg/reg_key.rs.html#164-177) calls the [`RegOpenKeyExW`](https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regopenkeyexw) function.
