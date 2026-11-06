//! Работа с процессами Windows: запуск клиента Roblox, обход блокировки
//! мульти-инстанса (закрытие синглтон-хэндлов ROBLOX_single* в чужом
//! процессе через перечисление таблицы хэндлов), трекинг и завершение.
//!
//! Все WinAPI объявлены вручную (ntdll / kernel32), чтобы не зависеть
//! от конкретной версии крейта `windows`.

#![allow(non_snake_case, non_camel_case_types, dead_code, non_upper_case_globals)]

/// Результат скана таблицы хэндлов одного процесса — для диагностики,
/// почему обход синглтона не нашёл/не закрыл объекты.
#[derive(Clone, Debug, Default)]
pub struct SingletonScan {
    /// Открылся ли процесс с правом PROCESS_DUP_HANDLE.
    pub open_ok: bool,
    /// Сколько записей таблицы принадлежит этому процессу.
    pub in_process: usize,
    /// Сколько из них Event/Mutant.
    pub typed: usize,
    /// Сколько синглтон-хэндлов закрыто.
    pub closed: usize,
    /// Примеры имён именованных Event/Mutant (до 6, для диагностики).
    pub names: Vec<String>,
}

#[cfg(windows)]
mod imp {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::SingletonScan;

    type HANDLE = isize;
    type HWND = isize;
    type LPARAM = isize;
    type DWORD = u32;
    type BOOL = i32;

    const FALSE: BOOL = 0;
    const PROCESS_DUP_HANDLE: DWORD = 0x0040;
    const PROCESS_QUERY_LIMITED_INFORMATION: DWORD = 0x1000;
    const PROCESS_TERMINATE: DWORD = 0x0001;
    const SYNCHRONIZE: DWORD = 0x0010_0000;
    const DUPLICATE_CLOSE_SOURCE: DWORD = 1;
    const DUPLICATE_SAME_ACCESS: DWORD = 2;
    const WAIT_TIMEOUT: DWORD = 0x0000_0102;
    const STILL_ACTIVE: DWORD = 259;
    const STATUS_INFO_LENGTH_MISMATCH: i32 = -1_073_741_820; // 0xC0000004
    const SystemExtendedHandleInformation: u32 = 64;
    const ObjectNameInformation: u32 = 1;
    const ObjectTypeInformation: u32 = 2;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct UnicodeString {
        length: u16,        // байтов (без нуля)
        maximum_length: u16,
        buffer: *mut u16,
    }

    /// SYSTEM_HANDLE_TABLE_ENTRY_INFO_EX (x64, 40 байт)
    #[repr(C)]
    struct HandleEntryEx {
        object: usize,
        process_id: usize,
        handle: usize,
        granted_access: u32,
        creator_back_trace_index: u16,
        object_type_index: u16,
        handle_attributes: u32,
        reserved: u32,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(desired_access: DWORD, inherit: BOOL, pid: DWORD) -> HANDLE;
        fn CloseHandle(h: HANDLE) -> BOOL;
        fn GetCurrentProcess() -> HANDLE;
        fn GetProcessTimes(
            h: HANDLE,
            created: *mut u64,
            exited: *mut u64,
            kernel: *mut u64,
            user: *mut u64,
        ) -> BOOL;
        fn DuplicateHandle(
            src_process: HANDLE,
            src_handle: HANDLE,
            dst_process: HANDLE,
            dst_handle: *mut HANDLE,
            desired_access: DWORD,
            inherit: BOOL,
            options: DWORD,
        ) -> BOOL;
        fn TerminateProcess(h: HANDLE, exit_code: u32) -> BOOL;
        fn WaitForSingleObject(h: HANDLE, ms: DWORD) -> DWORD;
        fn GetExitCodeProcess(h: HANDLE, code: *mut DWORD) -> BOOL;
        fn EnumWindows(
            cb: Option<unsafe extern "system" fn(HWND, LPARAM) -> BOOL>,
            lparam: LPARAM,
        ) -> BOOL;
        fn IsWindowVisible(h: HWND) -> BOOL;
        fn GetWindowTextW(h: HWND, buf: *mut u16, max_count: i32) -> i32;
        fn GetWindowTextLengthW(h: HWND) -> i32;
        fn GetWindowThreadProcessId(h: HWND, pid: *mut DWORD) -> DWORD;
        fn CreateMutexW(
            attrs: *mut core::ffi::c_void,
            initial_owner: BOOL,
            name: *const u16,
        ) -> HANDLE;
    }

    #[link(name = "ntdll")]
    extern "system" {
        fn NtQuerySystemInformation(
            class: u32,
            info: *mut u8,
            len: u32,
            ret_len: *mut u32,
        ) -> i32;
        fn NtQueryObject(
            handle: HANDLE,
            class: u32,
            info: *mut u8,
            len: u32,
            ret_len: *mut u32,
        ) -> i32;
    }

    /* ------------------------- Публичное API ------------------------- */

    /// Запустить RobloxPlayerBeta.exe; при `local_app_data` — подменить
    /// переменную окружения LOCALAPPDATA (изоляция данных инстанса).
    pub fn spawn_roblox(path: &str, local_app_data: Option<&Path>) -> Result<u32, String> {
        let mut cmd = std::process::Command::new(path);
        if let Some(dir) = local_app_data {
            cmd.env("LOCALAPPDATA", dir);
        }
        cmd.spawn()
            .map(|c| c.id())
            .map_err(|e| format!("не удалось запустить процесс: {e}"))
    }

    pub fn is_alive(pid: u32) -> bool {
        unsafe {
            let h = OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
            if h == 0 {
                // Нет доступа или процесс не существует — считаем живым только при доступе
                return false;
            }
            let wait = WaitForSingleObject(h, 0);
            CloseHandle(h);
            wait == WAIT_TIMEOUT
        }
    }

    pub fn exit_code(pid: u32) -> Option<u32> {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
            if h == 0 {
                return None;
            }
            let mut code: DWORD = 0;
            GetExitCodeProcess(h, &mut code);
            CloseHandle(h);
            if code == STILL_ACTIVE {
                None
            } else {
                Some(code)
            }
        }
    }

    /// Unix-ms время старта процесса (для attach свежих процессов при автологине).
    pub fn process_start_time(pid: u32) -> Option<i64> {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
            if h == 0 {
                return None;
            }
            let (mut created, mut exited, mut kernel, mut user): (u64, u64, u64, u64) =
                (0, 0, 0, 0);
            let ok = GetProcessTimes(h, &mut created, &mut exited, &mut kernel, &mut user);
            CloseHandle(h);
            if ok == 0 {
                return None;
            }
            // FILETIME (100 нс с 1601-01-01) -> unix ms
            const EPOCH_DIFF_100NS: u64 = 116_444_736_000_000_000;
            if created < EPOCH_DIFF_100NS {
                return None;
            }
            Some(((created - EPOCH_DIFF_100NS) / 10_000) as i64)
        }
    }

    /// Открыть URL-протокол через оболочку (roblox-player:, https: и т.п.).
    pub fn open_protocol(url: &str) -> Result<(), String> {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("не удалось открыть протокол roblox-player: {e}"))
    }

    pub fn terminate(pid: u32) -> Result<(), String> {
        unsafe {
            let h = OpenProcess(PROCESS_TERMINATE | SYNCHRONIZE, FALSE, pid);
            if h == 0 {
                return Err(format!("OpenProcess({pid}) не удался — нет доступа"));
            }
            let ok = TerminateProcess(h, 1);
            CloseHandle(h);
            if ok == 0 {
                Err(format!("TerminateProcess({pid}) не удался"))
            } else {
                Ok(())
            }
        }
    }

    /// Заголовок главного видимого окна процесса. У клиента Roblox может быть
    /// несколько видимых верхнеуровневых окон (главное + служебные), и порядок
    /// перечисления не гарантирован: в меню главное окно называется «Roblox»,
    /// в игре — «Название игры - Roblox». Поэтому собираем ВСЕ заголовки и
    /// берём самый информативный: отличный от «Roblox», если он существует.
    pub fn window_title(pid: u32) -> Option<String> {
        struct Ctx {
            pid: u32,
            titles: Vec<String>,
        }
        unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let ctx = &mut *(lparam as *mut Ctx);
            let mut win_pid: DWORD = 0;
            GetWindowThreadProcessId(hwnd, &mut win_pid);
            if win_pid != ctx.pid || IsWindowVisible(hwnd) == 0 {
                return 1; // продолжить перечисление
            }
            let len = GetWindowTextLengthW(hwnd);
            if len <= 0 {
                return 1;
            }
            let mut buf = vec![0u16; len as usize + 1];
            let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
            if n > 0 {
                ctx.titles.push(String::from_utf16_lossy(&buf[..n as usize]));
            }
            1
        }
        let mut ctx = Ctx { pid, titles: Vec::new() };
        unsafe {
            EnumWindows(Some(enum_cb), &mut ctx as *mut Ctx as LPARAM);
        }
        if ctx.titles.is_empty() {
            return None;
        }
        // Самый длинный заголовок, отличный от "roblox" (регистронезависимо)
        let best = ctx
            .titles
            .iter()
            .filter(|t| !t.trim().eq_ignore_ascii_case("roblox"))
            .max_by_key(|t| t.chars().count());
        match best {
            Some(t) => Some(t.clone()),
            // Все окна называются "Roblox" — значит, клиент в меню
            None => Some(ctx.titles[0].clone()),
        }
    }

    /* --------------------- Обход синглтон-хэндла --------------------- */

    fn read_us(buf: &[u8], us: &UnicodeString) -> String {
        if us.length == 0 || us.buffer.is_null() {
            return String::new();
        }
        let base = buf.as_ptr() as usize;
        let off = us.buffer as usize - base;
        let end = off + us.length as usize;
        if end > buf.len() {
            return String::new();
        }
        let chars: Vec<u16> = buf[off..end]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&chars)
    }

    /// Запросить имя/тип объекта хэндла через NtQueryObject.
    /// Возвращает пустую строку при неудаче (объект без имени и т.п.).
    fn query_object_info(handle: HANDLE, class: u32) -> Option<(Vec<u8>, usize)> {
        let mut size: u32 = 1024;
        for _ in 0..4 {
            let mut buf = vec![0u8; size as usize];
            let mut ret: u32 = 0;
            let status = unsafe {
                NtQueryObject(handle, class, buf.as_mut_ptr(), size, &mut ret)
            };
            if status >= 0 {
                return Some((buf, ret as usize));
            }
            if status == STATUS_INFO_LENGTH_MISMATCH {
                size *= 2;
                continue;
            }
            return None;
        }
        None
    }

    /// Закрыть все синглтон-хэндлы Roblox (Event/Mutant, в имени которых
    /// встречается `singleton`/`roblox_single`, регистронезависимо) в
    /// указанном процессе. Возвращает подробный результат скана.
    pub fn close_singleton_handles(pid: u32) -> SingletonScan {
        let mut scan = SingletonScan::default();
        unsafe {
            // 1) Снимок таблицы хэндлов всей системы
            let mut needed: u32 = 0;
            NtQuerySystemInformation(
                SystemExtendedHandleInformation,
                std::ptr::null_mut(),
                0,
                &mut needed,
            );
            let mut size = (needed as usize).saturating_mul(2).max(1 << 20);
            let mut buf: Vec<u8>;
            let mut ret;
            let mut ok_status;
            loop {
                buf = vec![0u8; size];
                ret = 0;
                ok_status = NtQuerySystemInformation(
                    SystemExtendedHandleInformation,
                    buf.as_mut_ptr(),
                    size as u32,
                    &mut ret,
                );
                if ok_status >= 0 {
                    break;
                }
                if ok_status == STATUS_INFO_LENGTH_MISMATCH {
                    size *= 2;
                    if size > (1 << 27) {
                        return scan;
                    }
                    continue;
                }
                return scan;
            }
            if buf.len() < 16 {
                return scan;
            }
            let count = usize::from_le_bytes([buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7]]);
            let entry_size = std::mem::size_of::<HandleEntryEx>();
            let entries_ptr = buf.as_ptr() as usize + 16;
            let avail = (buf.len() - 16) / entry_size;
            let count = count.min(avail);

            let target_proc = OpenProcess(PROCESS_DUP_HANDLE, FALSE, pid);
            scan.open_ok = target_proc != 0;
            if target_proc == 0 {
                // Нет доступа (защита процесса?) —диагностика покажет это
                return scan;
            }
            let me = GetCurrentProcess();

            for i in 0..count {
                let entry =
                    &*((entries_ptr + i * entry_size) as *const HandleEntryEx);
                if entry.process_id as u32 != pid {
                    continue;
                }
                scan.in_process += 1;
                let h_local = (entry.handle & 0xFFFF_FFFF) as isize;

                // Дубликат в наш процесс для чтения метаданных
                let mut dup: HANDLE = 0;
                let ok = DuplicateHandle(
                    target_proc,
                    h_local,
                    me,
                    &mut dup,
                    0,
                    FALSE,
                    DUPLICATE_SAME_ACCESS,
                );
                if ok == 0 {
                    continue;
                }

                // Тип объекта: интересуют только Event и Mutant (безопасно к запросу)
                let mut type_name = String::new();
                if let Some((tbuf, tlen)) = query_object_info(dup, ObjectTypeInformation) {
                    if tlen >= 24 && tbuf.len() >= 24 {
                        let us = UnicodeString {
                            length: u16::from_le_bytes([tbuf[0], tbuf[1]]),
                            maximum_length: u16::from_le_bytes([tbuf[2], tbuf[3]]),
                            buffer: usize::from_le_bytes([
                                tbuf[8], tbuf[9], tbuf[10], tbuf[11],
                                tbuf[12], tbuf[13], tbuf[14], tbuf[15],
                            ]) as *mut u16,
                        };
                        if us.length <= us.maximum_length {
                            type_name = read_us(&tbuf, &us);
                        }
                    }
                }

                if type_name != "Event" && type_name != "Mutant" {
                    CloseHandle(dup);
                    continue;
                }
                scan.typed += 1;

                // Имя объекта
                let mut obj_name = String::new();
                if let Some((nbuf, nlen)) = query_object_info(dup, ObjectNameInformation) {
                    if nlen >= 16 && nbuf.len() >= 16 {
                        let us = UnicodeString {
                            length: u16::from_le_bytes([nbuf[0], nbuf[1]]),
                            maximum_length: u16::from_le_bytes([nbuf[2], nbuf[3]]),
                            buffer: usize::from_le_bytes([
                                nbuf[8], nbuf[9], nbuf[10], nbuf[11],
                                nbuf[12], nbuf[13], nbuf[14], nbuf[15],
                            ]) as *mut u16,
                        };
                        if us.length <= us.maximum_length {
                            obj_name = read_us(&nbuf, &us);
                        }
                    }
                }
                if !obj_name.is_empty()
                    && scan.names.len() < 6
                    && !scan.names.contains(&obj_name)
                {
                    let shown: String = obj_name.chars().take(80).collect();
                    scan.names.push(shown);
                }
                CloseHandle(dup);

                // Закрываем ТОЛЬКО ROBLOX_singletonEvent. Мьютекс
                // ROBLOX_singletonMutex трогать НЕЛЬЗЯ: старый клиент
                // держит его как маркер «я первый» и периодически
                // перепроверяет — потеря мьютекса заставляет его
                // завершиться (это и убивало второй клиент).
                let lower = obj_name.to_ascii_lowercase();
                if lower.contains("singleton") && lower.contains("event") {
                    let ok = DuplicateHandle(
                        target_proc,
                        h_local,
                        target_proc,
                        std::ptr::null_mut(),
                        0,
                        FALSE,
                        DUPLICATE_CLOSE_SOURCE,
                    );
                    if ok != 0 {
                        scan.closed += 1;
                    }
                }
            }

            CloseHandle(target_proc);
        }
        scan
    }

    static SQUAT_OK: AtomicBool = AtomicBool::new(false);

    /// Перехват имени ROBLOX_singletonEvent: создаём в СВОЁМ процессе
    /// Mutant с этим именем и держим хэндл до конца работы приложения.
    /// После перехвата OpenEvent/CreateEvent Roblox по этому имени всегда
    /// падает с OBJECT_TYPE_MISMATCH — новому клиенту некого «сигналить»,
    /// и старые клиенты не закрываются. Ровно так работает проверенный
    /// обход (MultiBloxy, Zgoly/bloxstrap-multi-instance-integration).
    /// Возвращает true, если имя занято нашим объектом.
    pub fn squat_singleton_event() -> bool {
        if SQUAT_OK.load(Ordering::Relaxed) {
            return true;
        }
        unsafe {
            let mut name: Vec<u16> = "ROBLOX_singletonEvent"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            // Владение не важно: держим открытый хэндл, чтобы имя оставалось
            // занятым объектом типа Mutant (не Event — для Roblox несовместим)
            let h = CreateMutexW(std::ptr::null_mut(), FALSE, name.as_mut_ptr());
            if h != 0 {
                // Хэндл намеренно не закрываем — живёт вместе с процессом
                SQUAT_OK.store(true, Ordering::Relaxed);
                true
            } else {
                // Имя занято чужим объектом (реальный Event ещё жив) —
                // сначала закрываем его хэндлы у клиентов и повторяем
                false
            }
        }
    }

    /// Все PID процессов RobloxPlayerBeta.exe (для очистки путей запуска).
    pub fn roblox_pids() -> Vec<u32> {
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct ProcessEntry32W {
            size: DWORD,
            usage_count: DWORD,
            process_id: DWORD,
            default_heap_id: usize,
            module_base_id: DWORD,
            thread_count: DWORD,
            parent_process_id: DWORD,
            pc_pri_class_base: i32,
            dw_flags: DWORD,
            exe_file: [u16; 260],
        }

        #[link(name = "kernel32")]
        extern "system" {
            fn CreateToolhelp32Snapshot(flags: DWORD, pid: DWORD) -> HANDLE;
            fn Process32FirstW(snapshot: HANDLE, entry: *mut ProcessEntry32W) -> BOOL;
            fn Process32NextW(snapshot: HANDLE, entry: *mut ProcessEntry32W) -> BOOL;
        }

        const TH32CS_SNAPPROCESS: DWORD = 0x2;
        const INVALID_HANDLE_VALUE: HANDLE = -1;

        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE {
                return Vec::new();
            }
            let mut entry = ProcessEntry32W {
                size: std::mem::size_of::<ProcessEntry32W>() as DWORD,
                usage_count: 0,
                process_id: 0,
                default_heap_id: 0,
                module_base_id: 0,
                thread_count: 0,
                parent_process_id: 0,
                pc_pri_class_base: 0,
                dw_flags: 0,
                exe_file: [0; 260],
            };
            let mut pids = Vec::new();
            if Process32FirstW(snap, &mut entry) != 0 {
                loop {
                    let len = entry
                        .exe_file
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(260);
                    let name = String::from_utf16_lossy(&entry.exe_file[..len]);
                    if name.eq_ignore_ascii_case("robloxplayerbeta.exe") {
                        pids.push(entry.process_id);
                    }
                    if Process32NextW(snap, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snap);
            pids
        }
    }

    pub fn local_app_data() -> Option<PathBuf> {
        std::env::var("LOCALAPPDATA").ok().map(PathBuf::from)
    }
}

/* ------------------- Не-Windows заглушки ------------------- */

#[cfg(not(windows))]
mod imp {
    use std::path::Path;

    use super::SingletonScan;

    pub fn spawn_roblox(path: &str, _local_app_data: Option<&Path>) -> Result<u32, String> {
        std::process::Command::new(path)
            .spawn()
            .map(|c| c.id())
            .map_err(|e| format!("не удалось запустить процесс: {e}"))
    }

    pub fn is_alive(_pid: u32) -> bool {
        false
    }

    pub fn exit_code(_pid: u32) -> Option<u32> {
        None
    }

    pub fn process_start_time(_pid: u32) -> Option<i64> {
        None
    }

    pub fn open_protocol(_url: &str) -> Result<(), String> {
        Err("Доступно только в Windows".into())
    }

    pub fn terminate(_pid: u32) -> Result<(), String> {
        Err("Доступно только в Windows".into())
    }

    pub fn window_title(_pid: u32) -> Option<String> {
        None
    }

    pub fn close_singleton_handles(_pid: u32) -> SingletonScan {
        SingletonScan::default()
    }

    pub fn squat_singleton_event() -> bool {
        true
    }

    pub fn roblox_pids() -> Vec<u32> {
        Vec::new()
    }

    pub fn local_app_data() -> Option<std::path::PathBuf> {
        None
    }
}

pub use imp::*;

/// Каталог данных Roblox по умолчанию (для автопоиска клиента).
pub fn detect_client_path() -> Option<String> {
    let lad = local_app_data()?;
    let versions_dir = lad.join("Roblox").join("Versions");
    let mut best: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
    if let Ok(entries) = std::fs::read_dir(&versions_dir) {
        for entry in entries.flatten() {
            let dir = entry.path();
            let name = dir.file_name()?.to_string_lossy().to_string();
            if !name.starts_with("version-") {
                continue;
            }
            let exe = dir.join("RobloxPlayerBeta.exe");
            if exe.is_file() {
                let mtime = entry.metadata().and_then(|m| m.modified()).ok();
                if let Some(mt) = mtime {
                    if best.as_ref().map(|(t, _)| mt > *t).unwrap_or(true) {
                        best = Some((mt, exe));
                    }
                }
            }
        }
    }
    best.map(|(_, p)| p.to_string_lossy().into_owned())
}
