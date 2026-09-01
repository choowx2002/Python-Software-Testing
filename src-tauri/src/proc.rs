// Windows 下隐藏子进程的控制台窗口（CREATE_NO_WINDOW）。
// 控制台子系统程序（python / pip / git / winget / cmd 等）从 GUI 进程被 spawn
// 时，Windows 默认会新建一个 cmd 窗口，本模块统一关闭该行为。

#[cfg(windows)]
mod imp {
    pub(crate) const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    pub(crate) trait NoConsole {
        fn no_console(&mut self) -> &mut Self;
    }

    impl NoConsole for std::process::Command {
        fn no_console(&mut self) -> &mut Self {
            use std::os::windows::process::CommandExt;
            self.creation_flags(CREATE_NO_WINDOW)
        }
    }

    impl NoConsole for tokio::process::Command {
        fn no_console(&mut self) -> &mut Self {
            self.creation_flags(CREATE_NO_WINDOW)
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub(crate) trait NoConsole {
        fn no_console(&mut self) -> &mut Self;
    }

    impl NoConsole for std::process::Command {
        fn no_console(&mut self) -> &mut Self {
            self
        }
    }

    impl NoConsole for tokio::process::Command {
        fn no_console(&mut self) -> &mut Self {
            self
        }
    }
}

pub(crate) use imp::NoConsole;
