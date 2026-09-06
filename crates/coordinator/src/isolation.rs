use std::path::{Path, PathBuf};
use std::time::Duration;

use unveil_contracts::{
    IsolationCheck, IsolationDiagnosis, WorkerRequest, WorkerResponse, write_frame,
};

pub fn worker_path() -> PathBuf {
    if let Ok(path) = std::env::var("UNVEIL_WORKER") {
        return PathBuf::from(path);
    }
    let name = if cfg!(windows) {
        "unveil-worker.exe"
    } else {
        "unveil-worker"
    };
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join(name);
            if sibling.is_file() {
                return sibling;
            }
        }
    }
    let from_workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/debug")
        .join(name);
    if from_workspace.is_file() {
        return from_workspace;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/release")
        .join(name)
}

pub fn diagnose_isolation() -> IsolationDiagnosis {
    let platform = std::env::consts::OS.to_string();
    #[cfg(not(windows))]
    {
        return IsolationDiagnosis::fail_closed(
            platform,
            vec![IsolationCheck {
                id: "os_namespaces".into(),
                passed: false,
                detail: "Runtime OS is not the Windows lab target. Analysis stays fail-closed."
                    .into(),
            }],
        );
    }
    #[cfg(windows)]
    {
        windows_lab::diagnose(&platform)
    }
}

pub fn run_isolated(
    req: &WorkerRequest,
    body: &[u8],
    sandbox: &Path,
    canary: Option<&Path>,
    timeout: Duration,
) -> Result<WorkerResponse, unveil_contracts::IpcError> {
    #[cfg(not(windows))]
    {
        let _ = (req, body, sandbox, canary, timeout);
        Err(unveil_contracts::IpcError::new(
            unveil_contracts::ErrorCode::SandboxUnavailable,
            "error.sandbox_unavailable",
        ))
    }
    #[cfg(windows)]
    {
        windows_lab::run(req, body, sandbox, canary, timeout)
    }
}

#[cfg(windows)]
mod windows_lab {
    use super::*;
    use std::fs;
    use std::io::{Read, Write};
    use std::os::windows::io::FromRawHandle;
    use std::os::windows::raw::HANDLE as RawHANDLE;
    use std::ptr;

    use unveil_contracts::{ErrorCode, IpcError, IsolationSelfTestReport};

    const APP_CONTAINER_NAME: &str = "unveil.worker.v1";

    pub fn diagnose(platform: &str) -> IsolationDiagnosis {
        let worker = worker_path();
        let mut checks = vec![IsolationCheck {
            id: "worker_binary".into(),
            passed: worker.is_file(),
            detail: format!("{}", worker.display()),
        }];
        if !worker.is_file() {
            checks.push(IsolationCheck {
                id: "appcontainer_job".into(),
                passed: false,
                detail: "unveil-worker is not built beside the coordinator.".into(),
            });
            return IsolationDiagnosis::fail_closed(platform, checks);
        }
        let tmp = match tempfile_workspace() {
            Ok(p) => p,
            Err(err) => {
                checks.push(IsolationCheck {
                    id: "workspace".into(),
                    passed: false,
                    detail: err,
                });
                return IsolationDiagnosis::fail_closed(platform, checks);
            }
        };
        let sandbox = tmp.join("sandbox");
        let canary = tmp.join("canary.txt");
        let _ = fs::create_dir_all(&sandbox);
        let _ = fs::write(&canary, b"unveil-canary");
        let req = WorkerRequest::self_test(Some(canary.to_string_lossy().into_owned()));
        match run(&req, &[], &sandbox, Some(&canary), Duration::from_secs(8)) {
            Ok(resp) => match resp.self_test {
                Some(report) => {
                    push_report(&mut checks, &report);
                    if checks.iter().all(|c| c.passed) {
                        IsolationDiagnosis::ok(platform, checks)
                    } else {
                        IsolationDiagnosis::fail_closed(platform, checks)
                    }
                }
                None => {
                    checks.push(IsolationCheck {
                        id: "self_test_report".into(),
                        passed: false,
                        detail: "Worker returned no self-test report.".into(),
                    });
                    IsolationDiagnosis::fail_closed(platform, checks)
                }
            },
            Err(err) => {
                checks.push(IsolationCheck {
                    id: "spawn_isolated".into(),
                    passed: false,
                    detail: format!("{} ({:?})", err.message_key, err.code),
                });
                IsolationDiagnosis::fail_closed(platform, checks)
            }
        }
    }

    fn push_report(checks: &mut Vec<IsolationCheck>, report: &IsolationSelfTestReport) {
        checks.push(IsolationCheck {
            id: "child_spawn_denied".into(),
            passed: report.child_spawn_denied,
            detail: report.notes.join(" | "),
        });
        checks.push(IsolationCheck {
            id: "network_denied".into(),
            passed: report.network_denied,
            detail: "AppContainer without internetClient must fail connect.".into(),
        });
        checks.push(IsolationCheck {
            id: "extra_read_denied".into(),
            passed: report.extra_read_denied,
            detail: "Canary outside the granted sandbox must be unreadable.".into(),
        });
        checks.push(IsolationCheck {
            id: "extra_write_denied".into(),
            passed: report.extra_write_denied,
            detail: "Canary directory must not be writable.".into(),
        });
    }

    fn tempfile_workspace() -> Result<PathBuf, String> {
        let base = std::env::temp_dir().join(format!("unveil-iso-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&base).map_err(|e| e.to_string())?;
        Ok(base)
    }

    pub fn run(
        req: &WorkerRequest,
        body: &[u8],
        sandbox: &Path,
        canary: Option<&Path>,
        timeout: Duration,
    ) -> Result<WorkerResponse, IpcError> {
        native::spawn_and_exchange(req, body, sandbox, canary, timeout)
    }

    mod native {
        use super::*;
        use std::mem::{size_of, zeroed};

        use windows::core::{PCWSTR, PWSTR};
        use windows::Win32::Foundation::{
            CloseHandle, HANDLE, HANDLE_FLAGS, WAIT_OBJECT_0, WAIT_TIMEOUT,
        };
        use windows::Win32::Security::{
            ACL, DACL_SECURITY_INFORMATION, PSID, SECURITY_CAPABILITIES,
            SUB_CONTAINERS_AND_OBJECTS_INHERIT,
        };
        use windows::Win32::Security::Authorization::{
            SetEntriesInAclW, SetNamedSecurityInfoW, EXPLICIT_ACCESS_W, SE_FILE_OBJECT, SET_ACCESS,
            TRUSTEE_IS_SID, TRUSTEE_IS_WELL_KNOWN_GROUP, TRUSTEE_W,
        };
        use windows::Win32::Security::Isolation::{
            CreateAppContainerProfile, DeriveAppContainerSidFromAppContainerName,
        };
        use windows::Win32::Storage::FileSystem::{
            FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_TRAVERSE,
        };
        use windows::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
            JobObjectExtendedLimitInformation, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOB_OBJECT_LIMIT_PROCESS_MEMORY,
        };
        use windows::Win32::System::Pipes::CreatePipe;
        use windows::Win32::System::Threading::{
            CreateProcessW, DeleteProcThreadAttributeList, InitializeProcThreadAttributeList,
            ResumeThread, TerminateProcess, UpdateProcThreadAttribute, WaitForSingleObject,
            CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, EXTENDED_STARTUPINFO_PRESENT,
            LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
            STARTF_USESTDHANDLES, STARTUPINFOEXW, CREATE_NO_WINDOW,
        };
        use windows::Win32::System::Memory::{GetProcessHeap, HeapAlloc, HeapFree, HEAP_ZERO_MEMORY};

        pub fn spawn_and_exchange(
            req: &WorkerRequest,
            body: &[u8],
            sandbox: &Path,
            _canary: Option<&Path>,
            timeout: Duration,
        ) -> Result<WorkerResponse, IpcError> {
            fs::create_dir_all(sandbox)
                .map_err(|_| IpcError::new(ErrorCode::PermissionDenied, "error.permission_denied"))?;
            unsafe { spawn_inner(req, body, sandbox, timeout) }
        }

        unsafe fn spawn_inner(
            req: &WorkerRequest,
            body: &[u8],
            sandbox: &Path,
            timeout: Duration,
        ) -> Result<WorkerResponse, IpcError> {
            let worker = worker_path();
            if !worker.is_file() {
                return Err(IpcError::new(
                    ErrorCode::SandboxUnavailable,
                    "error.sandbox_unavailable",
                ));
            }
            let sid = appcontainer_sid()?;
            grant_dir(sandbox, sid)?;

            let sa = windows::Win32::Security::SECURITY_ATTRIBUTES {
                nLength: size_of::<windows::Win32::Security::SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: ptr::null_mut(),
                bInheritHandle: true.into(),
            };
            let mut stdin_r = HANDLE::default();
            let mut stdin_w = HANDLE::default();
            let mut stdout_r = HANDLE::default();
            let mut stdout_w = HANDLE::default();
            CreatePipe(&mut stdin_r, &mut stdin_w, Some(&sa), 0).map_err(win_err)?;
            CreatePipe(&mut stdout_r, &mut stdout_w, Some(&sa), 0).map_err(win_err)?;
            make_non_inheritable(stdin_w)?;
            make_non_inheritable(stdout_r)?;

            let job = CreateJobObjectW(None, PCWSTR::null()).map_err(win_err)?;
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = zeroed();
            limits.BasicLimitInformation.LimitFlags =
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                    | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
                    | JOB_OBJECT_LIMIT_PROCESS_MEMORY;
            limits.BasicLimitInformation.ActiveProcessLimit = 1;
            limits.ProcessMemoryLimit = 512 * 1024 * 1024;
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
            .map_err(win_err)?;

            let mut cap = SECURITY_CAPABILITIES {
                AppContainerSid: sid,
                Capabilities: ptr::null_mut(),
                CapabilityCount: 0,
                Reserved: 0,
            };

            let mut attr_size = usize::default();
            let _ = InitializeProcThreadAttributeList(None, 1, Some(0), &mut attr_size);
            let heap = GetProcessHeap().map_err(win_err)?;
            let attr_mem = HeapAlloc(heap, HEAP_ZERO_MEMORY, attr_size);
            if attr_mem.is_null() {
                return Err(IpcError::new(ErrorCode::SandboxUnavailable, "error.sandbox_unavailable"));
            }
            let attr_list = LPPROC_THREAD_ATTRIBUTE_LIST(attr_mem);
            InitializeProcThreadAttributeList(Some(attr_list), 1, Some(0), &mut attr_size).map_err(win_err)?;
            UpdateProcThreadAttribute(
                attr_list,
                0,
                PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
                Some((&mut cap) as *mut _ as *mut _),
                size_of::<SECURITY_CAPABILITIES>(),
                None,
                None,
            )
            .map_err(win_err)?;

            let mut si: STARTUPINFOEXW = zeroed();
            si.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
            si.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
            si.StartupInfo.hStdInput = stdin_r;
            si.StartupInfo.hStdOutput = stdout_w;
            si.StartupInfo.hStdError = stdout_w;
            si.lpAttributeList = attr_list;

            let mut cmd = to_wide(&format!("\"{}\" --isolated", worker.display()));
            let cwd = to_wide(&sandbox.to_string_lossy());
            let mut pi = PROCESS_INFORMATION::default();
            let created = CreateProcessW(
                PCWSTR::null(),
                Some(PWSTR(cmd.as_mut_ptr())),
                None,
                None,
                true,
                CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT | CREATE_NO_WINDOW | CREATE_UNICODE_ENVIRONMENT,
                None,
                PCWSTR(cwd.as_ptr()),
                &si.StartupInfo,
                &mut pi,
            );
            let _ = CloseHandle(stdin_r);
            let _ = CloseHandle(stdout_w);
            DeleteProcThreadAttributeList(attr_list);
            let _ = HeapFree(heap, Default::default(), Some(attr_mem));
            created.map_err(win_err)?;
            AssignProcessToJobObject(job, pi.hProcess).map_err(win_err)?;
            ResumeThread(pi.hThread);

            let mut writer = std::fs::File::from_raw_handle(stdin_w.0 as RawHANDLE);
            let header = serde_json::to_vec(req)
                .map_err(|_| IpcError::new(ErrorCode::SchemaMismatch, "error.schema_mismatch"))?;
            write_frame(&mut writer, &header)
                .map_err(|_| IpcError::new(ErrorCode::WorkerCrashed, "error.worker_crashed"))?;
            if req.op != "self_test" {
                writer
                    .write_all(&(body.len() as u32).to_le_bytes())
                    .and_then(|_| writer.write_all(body))
                    .and_then(|_| writer.flush())
                    .map_err(|_| IpcError::new(ErrorCode::WorkerCrashed, "error.worker_crashed"))?;
            }
            drop(writer);

            let wait = WaitForSingleObject(pi.hProcess, timeout.as_millis() as u32);
            if wait == WAIT_TIMEOUT {
                let _ = TerminateProcess(pi.hProcess, 1);
                cleanup(pi, job);
                return Err(IpcError::new(ErrorCode::LimitReached, "error.limit_reached"));
            }
            if wait != WAIT_OBJECT_0 {
                cleanup(pi, job);
                return Err(IpcError::new(ErrorCode::WorkerCrashed, "error.worker_crashed"));
            }

            let mut reader = std::fs::File::from_raw_handle(stdout_r.0 as RawHANDLE);
            let mut len_buf = [0u8; 4];
            reader
                .read_exact(&mut len_buf)
                .map_err(|_| IpcError::new(ErrorCode::WorkerCrashed, "error.worker_crashed"))?;
            let len = u32::from_le_bytes(len_buf) as usize;
            if len == 0 || len > 8 * 1024 * 1024 {
                cleanup(pi, job);
                return Err(IpcError::new(ErrorCode::SchemaMismatch, "error.schema_mismatch"));
            }
            let mut buf = vec![0u8; len];
            reader
                .read_exact(&mut buf)
                .map_err(|_| IpcError::new(ErrorCode::WorkerCrashed, "error.worker_crashed"))?;
            cleanup(pi, job);
            serde_json::from_slice(&buf)
                .map_err(|_| IpcError::new(ErrorCode::SchemaMismatch, "error.schema_mismatch"))
        }

        fn cleanup(pi: PROCESS_INFORMATION, job: HANDLE) {
            unsafe {
                let _ = CloseHandle(pi.hThread);
                let _ = CloseHandle(pi.hProcess);
                let _ = CloseHandle(job);
            }
        }

        fn make_non_inheritable(handle: HANDLE) -> Result<(), IpcError> {
            set_no_inherit(handle)
        }

        fn set_no_inherit(handle: HANDLE) -> Result<(), IpcError> {
            use windows::Win32::Foundation::{HANDLE_FLAG_INHERIT, SetHandleInformation};
            unsafe {
                SetHandleInformation(handle, HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0))
                    .map_err(win_err)
            }
        }

        fn appcontainer_sid() -> Result<PSID, IpcError> {
            unsafe {
                let name = windows::core::HSTRING::from(APP_CONTAINER_NAME);
                match CreateAppContainerProfile(
                    &name,
                    &windows::core::HSTRING::from("UNVEIL Worker"),
                    &windows::core::HSTRING::from("Isolated static analysis"),
                    None,
                ) {
                    Ok(sid) => Ok(sid),
                    Err(_) => DeriveAppContainerSidFromAppContainerName(&name).map_err(win_err),
                }
            }
        }

        fn grant_dir(path: &Path, sid: PSID) -> Result<(), IpcError> {
            unsafe {
                let access = EXPLICIT_ACCESS_W {
                    grfAccessPermissions: (FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0 | FILE_TRAVERSE.0),
                    grfAccessMode: SET_ACCESS,
                    grfInheritance: SUB_CONTAINERS_AND_OBJECTS_INHERIT,
                    Trustee: TRUSTEE_W {
                        pMultipleTrustee: ptr::null_mut(),
                        MultipleTrusteeOperation: Default::default(),
                        TrusteeForm: TRUSTEE_IS_SID,
                        TrusteeType: TRUSTEE_IS_WELL_KNOWN_GROUP,
                        ptstrName: PWSTR(sid.0 as *mut u16),
                    },
                };
                let mut new_acl: *mut ACL = ptr::null_mut();
                let acl_err = SetEntriesInAclW(Some(&[access]), None, &mut new_acl);
                if acl_err.0 != 0 {
                    return Err(IpcError::new(ErrorCode::SandboxUnavailable, "error.sandbox_unavailable"));
                }
                let wide = to_wide(&path.to_string_lossy());
                let set_err = SetNamedSecurityInfoW(
                    PCWSTR(wide.as_ptr()),
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION,
                    None,
                    None,
                    Some(&*new_acl),
                    None,
                );
                if set_err.0 != 0 {
                    return Err(IpcError::new(ErrorCode::SandboxUnavailable, "error.sandbox_unavailable"));
                }
            }
            Ok(())
        }

        fn to_wide(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(std::iter::once(0)).collect()
        }

        fn win_err(_: windows::core::Error) -> IpcError {
            IpcError::new(ErrorCode::SandboxUnavailable, "error.sandbox_unavailable")
        }
    }
}
