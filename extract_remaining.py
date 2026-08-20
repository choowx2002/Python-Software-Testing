import re

with open('src-tauri/src/commands_original.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

# 1. Extract suites.rs (lines 842-988, 1-based)
suites_lines = lines[841:988]
suites_content = ''.join(suites_lines)
# Fix run_tests_core call
suites_content = suites_content.replace(
    '    run_tests_core(\n',
    '    crate::commands::execution::run_tests_core(\n'
)

# Add header
suites_header = '''use crate::state::{process_alive, terminate_pid, AppState};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::process::Command as AsyncCommand;
use std::process::Stdio;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, FilePath};
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

use super::*;
'''

with open('src-tauri/src/commands/suites.rs', 'w', encoding='utf-8') as f:
    f.write(suites_header + suites_content)

# 2. Extract files.rs
# open_in_file_manager: 1178-1225
# open_file: 1839-1872
# reveal_in_folder: 1878-1924
files_parts = lines[1177:1225] + lines[1838:1872] + lines[1877:1924]
files_content = ''.join(files_parts)

files_header = '''use crate::state::{process_alive, terminate_pid, AppState};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::process::Command as AsyncCommand;
use std::process::Stdio;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, FilePath};
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

use super::*;
'''

with open('src-tauri/src/commands/files.rs', 'w', encoding='utf-8') as f:
    f.write(files_header + files_content)

# 3. Extract generation.rs (1249-1778)
gen_lines = lines[1248:1778]
gen_content = ''.join(gen_lines)

gen_header = '''use crate::state::{process_alive, terminate_pid, AppState};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::process::Command as AsyncCommand;
use std::process::Stdio;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, FilePath};
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

use super::*;
'''

with open('src-tauri/src/commands/generation.rs', 'w', encoding='utf-8') as f:
    f.write(gen_header + gen_content)

print('Done!')
