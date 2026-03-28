use crate::brain::Gateway;
use crate::config::Settings;
use crate::eyes::{capture, vision};
use crate::hands;
use crate::memory::Database;
use crate::pipeline::executor;
use crate::types::*;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::Emitter;
use tracing::{info, warn};

const MAX_STEPS: u32 = 30;
const MAX_RETRIES: u32 = 2;

const PC_MASTER_PROMPT: &str = r#"You are AgentOS, an AI that CONTROLS a Windows 11 PC via PowerShell. You receive a task and you MUST execute it.

Respond with a JSON object in ONE of these formats:

{"mode": "command", "commands": ["command1", "command2"], "explanation": "brief description"}
{"mode": "screen", "reason": "why I need to see the screen"}
{"mode": "done", "summary": "what was accomplished", "output": "data for the user"}
{"mode": "need_info", "question": "what you need to know"}
{"mode": "chat", "response": "conversational answer"}

CRITICAL POWERSHELL RULES:
1. ALL commands in the "commands" array are joined with "; " and run as ONE script
2. Use SINGLE QUOTES for literal strings: 'text here'
3. Use DOUBLE QUOTES only when you need variable expansion: "$env:USERPROFILE\path"
4. NEVER use backtick-escaping of $ inside double quotes. Instead use single quotes or -f format operator
5. For iterating drives, use: Get-PSDrive -PSProvider FileSystem | ForEach-Object { $d = $_.Root; ... }
6. Use -ErrorAction SilentlyContinue liberally to prevent non-critical errors from crashing the script
7. Always end with Write-Output to produce visible results
8. When a command might produce no output, add a fallback: Write-Output 'Done'
9. For hidden files: Get-ChildItem -Force -Hidden
10. For all drives: Get-PSDrive -PSProvider FileSystem gives you C, D, etc.
11. NEVER use $drive:\ syntax — it's invalid. Use Join-Path or string concatenation: "$($d.Root)folder"
12. Wrap complex scripts in try/catch: try { ... } catch { Write-Output "Error: $_" }
13. Use Format-Table -AutoSize or Format-List for readable output
14. For file sizes use: @{N='Size(MB)';E={[math]::Round($_.Length/1MB,2)}}
15. Start-Process for GUI apps (they get their own window, don't block)
16. For Windows Settings use: Start-Process 'ms-settings:page'

WHAT YOU CAN DO:
- Open ANY app, folder, URL, Windows settings page
- Create, read, edit, move, copy, rename, delete files and folders
- Organize files by type, date, size, project
- Search files by name, extension, content (Select-String)
- Compress/extract archives (Compress-Archive, Expand-Archive)
- Download files (Invoke-WebRequest)
- System info: disks, memory, CPU, network, battery, OS version
- Network: ping, DNS, adapters, wifi, IP config
- Processes: list, kill, start, monitor
- Services: list, start, stop, restart
- Registry: read, write (HKCU, HKLM)
- Installed programs: Get-Package, Get-WmiObject Win32_Product
- Windows config: dark/light mode, wallpaper, display, sound, bluetooth
- Environment variables: get, set, remove
- Scheduled tasks: create, list, remove
- User accounts: list, info
- Clipboard: read, write
- Date/time: get, set timezone
- Power: sleep, restart, shutdown (ONLY if explicitly asked)
- PDF text extraction: requires specific approach per tool available
- Excel/CSV: Import-Csv, Export-Csv, basic data manipulation
- JSON/XML: ConvertFrom-Json, ConvertTo-Json, [xml] cast
- Web scraping: Invoke-WebRequest with HTML parsing
- Git operations: git status, git log, git pull, etc.
- Docker: docker ps, docker-compose, etc. (if installed)
- Python/Node scripts: python -c "code", node -e "code" (if installed)

WHEN TO USE CHAT MODE:
- Abstract questions: "what is machine learning", "explain docker"
- Opinions: "what's the best programming language"
- General knowledge that doesn't require PC access

WHEN TO USE SCREEN MODE:
- Click buttons in GUI apps that can't be automated via command line
- Read visual content (images, diagrams)
- Interact with web apps in the browser
- Fill forms in applications

EXAMPLES:

"abre la carpeta descargas"
{"mode":"command","commands":["Start-Process explorer.exe \"$env:USERPROFILE\\Downloads\""],"explanation":"Opening Downloads"}

"que archivos hay en mi escritorio"
{"mode":"command","commands":["Get-ChildItem \"$env:USERPROFILE\\Desktop\" | Format-Table Name, Length, LastWriteTime -AutoSize"],"explanation":"Listing Desktop"}

"cuanto espacio libre tengo"
{"mode":"command","commands":["Get-PSDrive -PSProvider FileSystem | Where-Object {$_.Used -gt 0} | Select-Object Name, @{N='Used(GB)';E={[math]::Round($_.Used/1GB,2)}}, @{N='Free(GB)';E={[math]::Round($_.Free/1GB,2)}}, @{N='Total(GB)';E={[math]::Round(($_.Used+$_.Free)/1GB,2)}} | Format-Table -AutoSize"],"explanation":"Disk space info"}

"tengo archivos ocultos?"
{"mode":"command","commands":["$drives = (Get-PSDrive -PSProvider FileSystem).Root; $total = 0; foreach($r in $drives){ try{ $hidden = Get-ChildItem -Path $r -Hidden -File -ErrorAction SilentlyContinue | Measure-Object; $total += $hidden.Count; Write-Output \"$r : $($hidden.Count) hidden files\" }catch{} }; Write-Output \"`nTotal hidden files: $total\""],"explanation":"Counting hidden files per drive"}

"crea un archivo en el escritorio que diga hola"
{"mode":"command","commands":["Set-Content -Path \"$env:USERPROFILE\\Desktop\\hola.txt\" -Value 'Hola Mundo' -Encoding UTF8; Write-Output 'Created hola.txt on Desktop'"],"explanation":"Creating text file"}

"organiza mis descargas por tipo"
{"mode":"command","commands":["$dl = \"$env:USERPROFILE\\Downloads\"; $map = @{Imagenes='.jpg','.jpeg','.png','.gif','.bmp','.svg','.webp';Documentos='.pdf','.doc','.docx','.xls','.xlsx','.ppt','.pptx','.txt';Videos='.mp4','.avi','.mkv','.mov';Audio='.mp3','.wav','.flac','.aac';Archivos='.zip','.rar','.7z';Codigo='.py','.js','.ts','.html','.css','.rs'}; foreach($cat in $map.Keys){ $dir = Join-Path $dl $cat; if(!(Test-Path $dir)){New-Item -ItemType Directory -Path $dir -Force | Out-Null}; foreach($ext in $map[$cat]){ Get-ChildItem -Path \"$dl\\*$ext\" -File -ErrorAction SilentlyContinue | Move-Item -Destination $dir -Force } }; Get-ChildItem $dl -Directory | ForEach-Object { Write-Output \"$($_.Name): $((Get-ChildItem $_.FullName -File).Count) files\" }"],"explanation":"Organizing by file type"}

"pon modo oscuro"
{"mode":"command","commands":["Set-ItemProperty -Path 'HKCU:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize' -Name 'AppsUseLightTheme' -Value 0; Set-ItemProperty -Path 'HKCU:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize' -Name 'SystemUsesLightTheme' -Value 0; Write-Output 'Dark mode enabled'"],"explanation":"Enabling dark mode"}

"cual es el archivo mas grande en descargas"
{"mode":"command","commands":["Get-ChildItem \"$env:USERPROFILE\\Downloads\" -File -Recurse -ErrorAction SilentlyContinue | Sort-Object Length -Descending | Select-Object -First 10 Name, @{N='Size(MB)';E={[math]::Round($_.Length/1MB,2)}}, LastWriteTime | Format-Table -AutoSize"],"explanation":"Top 10 largest files in Downloads"}

"que programas tengo instalados"
{"mode":"command","commands":["Get-Package -ErrorAction SilentlyContinue | Select-Object Name, Version | Sort-Object Name | Format-Table -AutoSize"],"explanation":"Listing installed packages"}

"busca archivos que contengan 'password' en documentos"
{"mode":"command","commands":["Get-ChildItem \"$env:USERPROFILE\\Documents\" -File -Recurse -Include '*.txt','*.doc','*.csv','*.log' -ErrorAction SilentlyContinue | Select-String -Pattern 'password' -SimpleMatch | Select-Object -First 20 Path, LineNumber, Line | Format-Table -AutoSize"],"explanation":"Searching file contents"}

"abre youtube"
{"mode":"command","commands":["Start-Process 'https://www.youtube.com'"],"explanation":"Opening YouTube in default browser"}

"que hora es"
{"mode":"command","commands":["Get-Date -Format 'dddd, dd MMMM yyyy HH:mm:ss'"],"explanation":"Current date and time"}

"informacion de mi sistema"
{"mode":"command","commands":["$os = Get-CimInstance Win32_OperatingSystem; $cpu = Get-CimInstance Win32_Processor; $ram = [math]::Round($os.TotalVisibleMemorySize/1MB,1); $freeRam = [math]::Round($os.FreePhysicalMemory/1MB,1); Write-Output \"OS: $($os.Caption) $($os.Version)`nCPU: $($cpu.Name)`nRAM: $($freeRam)GB free / $($ram)GB total`nComputer: $env:COMPUTERNAME`nUser: $env:USERNAME\""],"explanation":"System information summary"}
"#;

const RETRY_PROMPT: &str = r#"The PowerShell command you generated FAILED with this error:

```
{ERROR}
```

The original command was:
```
{COMMAND}
```

Fix the command. Common issues:
- Don't use $variable:\ syntax (invalid). Use Join-Path or "$($var)\path"
- Use single quotes for literals, double quotes only for variable expansion
- Add -ErrorAction SilentlyContinue for non-critical operations
- Wrap in try/catch for robustness

Respond with the SAME JSON format: {"mode":"command","commands":[...],"explanation":"..."}
"#;

// ═══════════════════════════════════════════════════════════════════

pub async fn run_task(
    task_id: &str,
    description: &str,
    settings: &Settings,
    kill_switch: &Arc<AtomicBool>,
    screenshots_dir: &Path,
    db_path: &Path,
    app_handle: &tauri::AppHandle,
) -> Result<TaskExecutionResult, String> {
    let start = Instant::now();
    let gateway = Gateway::new(settings);
    let mut step_history: Vec<StepRecord> = Vec::new();
    let mut accumulated_output = String::new();

    info!(task_id, description, "Starting PC task execution");

    let _ = app_handle.emit("agent:step_started", serde_json::json!({
        "task_id": task_id, "step_number": 0,
    }));

    // Ask LLM what to do
    let plan = gateway
        .complete_with_system(description, Some(PC_MASTER_PROMPT), settings)
        .await
        .map_err(|e| {
            save_task_output(db_path, task_id, &format!("LLM error: {}", e));
            update_task_status(db_path, task_id, "failed");
            e
        })?;

    let plan_text = plan.content.trim().to_string();
    info!(task_id, plan = %plan_text, "LLM response");

    let plan_json = match extract_json(&plan_text) {
        Some(json) => json,
        None => {
            // LLM didn't return JSON — show raw response as chat
            save_task_output(db_path, task_id, &plan_text);
            update_task_status(db_path, task_id, "completed");
            accumulated_output = plan_text;
            // Jump to emit
            let duration_ms = start.elapsed().as_millis() as u64;
            let _ = app_handle.emit("agent:task_completed", serde_json::json!({
                "task_id": task_id, "success": true, "output": accumulated_output,
                "steps": 0, "duration_ms": duration_ms,
            }));
            return Ok(TaskExecutionResult {
                task_id: task_id.to_string(), success: true,
                steps: vec![], total_cost: 0.0, duration_ms,
            });
        }
    };

    let mode = plan_json["mode"].as_str().unwrap_or("command");

    match mode {
        "command" => {
            let commands = plan_json["commands"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();
            let explanation = plan_json["explanation"].as_str().unwrap_or("");

            if commands.is_empty() {
                accumulated_output = if !explanation.is_empty() {
                    explanation.to_string()
                } else {
                    "No commands generated".to_string()
                };
                save_task_output(db_path, task_id, &accumulated_output);
                update_task_status(db_path, task_id, "completed");
            } else {
                let full_script = commands.join("; ");
                info!(task_id, script = %full_script, "Executing PowerShell");

                let _ = app_handle.emit("agent:step_started", serde_json::json!({
                    "task_id": task_id, "step_number": 1, "description": explanation,
                }));

                let is_gui = full_script.to_lowercase().contains("start-process");
                let timeout = if is_gui { 30 } else { settings.cli_timeout };

                // Execute with retry on failure
                let (success, output) = execute_with_retry(
                    &full_script, timeout, description, &gateway, settings, task_id,
                ).await;

                if is_gui {
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                }

                let screenshot_path = take_screenshot(screenshots_dir).await;
                let sp = screenshot_path.as_ref().map(|p| p.to_string_lossy().to_string());

                let action = AgentAction::RunCommand { command: full_script.clone(), shell: ShellType::PowerShell };
                let result = ExecutionResult {
                    method: ExecutionMethod::Terminal, success,
                    output: Some(output.clone()), screenshot_path: sp.clone(),
                    duration_ms: start.elapsed().as_millis() as u64,
                };
                save_step(db_path, task_id, 1, &action, &screenshot_path.unwrap_or_default(), &result);

                let _ = app_handle.emit("agent:step_completed", serde_json::json!({
                    "task_id": task_id, "step_number": 1, "success": success,
                }));

                step_history.push(StepRecord {
                    step_number: 1,
                    action: AgentAction::TaskComplete { summary: output.clone() },
                    result, screenshot_path: sp,
                });

                accumulated_output = output;
                update_task_status(db_path, task_id, if success { "completed" } else { "failed" });
            }
        }

        "screen" => {
            let reason = plan_json["reason"].as_str().unwrap_or("Complex UI task");
            info!(task_id, reason, "Entering vision mode");

            for step_number in 1..=MAX_STEPS {
                if kill_switch.load(Ordering::Relaxed) {
                    update_task_status(db_path, task_id, "killed");
                    return Err("Kill switch activated".to_string());
                }

                let screenshot = tokio::task::spawn_blocking({
                    let sd = screenshots_dir.to_path_buf();
                    move || {
                        let data = capture::capture_full_screen().map_err(|e| e.to_string())?;
                        let path = capture::save_screenshot(&data, &sd).map_err(|e| e.to_string())?;
                        let b64 = capture::to_base64_jpeg(&data, 80).map_err(|e| e.to_string())?;
                        Ok::<_, String>((path, b64))
                    }
                }).await.map_err(|e| e.to_string())??;

                let (screenshot_path, screenshot_b64) = screenshot;
                let _ = app_handle.emit("agent:step_started", serde_json::json!({
                    "task_id": task_id, "step_number": step_number,
                }));

                let action = match vision::plan_next_action(
                    &screenshot_b64, description, &step_history, settings, &gateway,
                ).await {
                    Ok(a) => a,
                    Err(e) => {
                        warn!(task_id, error = %e, "Vision failed");
                        accumulated_output = format!("Vision error: {}", e);
                        update_task_status(db_path, task_id, "failed");
                        break;
                    }
                };

                if let AgentAction::TaskComplete { ref summary } = action {
                    accumulated_output = summary.clone();
                    update_task_status(db_path, task_id, "completed");
                    step_history.push(StepRecord {
                        step_number, action,
                        result: ExecutionResult {
                            method: ExecutionMethod::Screen, success: true,
                            output: Some(accumulated_output.clone()),
                            screenshot_path: Some(screenshot_path.to_string_lossy().to_string()),
                            duration_ms: 0,
                        },
                        screenshot_path: Some(screenshot_path.to_string_lossy().to_string()),
                    });
                    break;
                }

                let result = match executor::execute(&action, settings.cli_timeout, kill_switch).await {
                    Ok(r) => r,
                    Err(e) => ExecutionResult {
                        method: ExecutionMethod::Screen, success: false,
                        output: Some(e), screenshot_path: None, duration_ms: 0,
                    },
                };

                save_step(db_path, task_id, step_number, &action, &screenshot_path, &result);
                let _ = app_handle.emit("agent:step_completed", serde_json::json!({
                    "task_id": task_id, "step_number": step_number, "success": result.success,
                }));

                step_history.push(StepRecord {
                    step_number, action, result,
                    screenshot_path: Some(screenshot_path.to_string_lossy().to_string()),
                });

                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }

            if !step_history.last().map(|s| matches!(s.action, AgentAction::TaskComplete { .. })).unwrap_or(false) {
                if accumulated_output.is_empty() { accumulated_output = "Task did not complete within step limit".to_string(); }
                update_task_status(db_path, task_id, "failed");
            }
        }

        "done" => {
            accumulated_output = plan_json["output"].as_str()
                .or(plan_json["summary"].as_str())
                .unwrap_or("Task completed").to_string();
            update_task_status(db_path, task_id, "completed");
        }

        "need_info" => {
            accumulated_output = plan_json["question"].as_str()
                .unwrap_or("I need more information").to_string();
            update_task_status(db_path, task_id, "completed");
        }

        "chat" => {
            accumulated_output = plan_json["response"].as_str()
                .unwrap_or("I'm not sure how to help with that.").to_string();
            update_task_status(db_path, task_id, "completed");
        }

        _ => {
            accumulated_output = format!("Unknown response mode: {}", mode);
            update_task_status(db_path, task_id, "failed");
        }
    }

    // Always save output
    if !accumulated_output.is_empty() {
        save_task_output(db_path, task_id, &accumulated_output);
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let success = step_history.last().map(|s| s.result.success).unwrap_or(!accumulated_output.is_empty());

    let _ = app_handle.emit("agent:task_completed", serde_json::json!({
        "task_id": task_id, "success": success, "output": accumulated_output,
        "steps": step_history.len(), "duration_ms": duration_ms,
    }));

    Ok(TaskExecutionResult {
        task_id: task_id.to_string(), success,
        steps: step_history, total_cost: 0.0, duration_ms,
    })
}

// ═══════════════════════════════════════════════════════════════════
// EXECUTE WITH AUTO-RETRY
// ═══════════════════════════════════════════════════════════════════

async fn execute_with_retry(
    script: &str,
    timeout: u64,
    original_task: &str,
    gateway: &Gateway,
    settings: &Settings,
    task_id: &str,
) -> (bool, String) {
    let mut current_script = script.to_string();

    for attempt in 0..=MAX_RETRIES {
        let exec_result = hands::cli::run_powershell(&current_script, timeout).await;

        match exec_result {
            Ok(output) => {
                if output.exit_code == 0 {
                    // Success
                    let out = if !output.stdout.trim().is_empty() {
                        output.stdout
                    } else {
                        "Command executed successfully.".to_string()
                    };
                    return (true, out);
                }

                // Command failed — try to auto-correct
                let error_msg = if !output.stderr.trim().is_empty() {
                    output.stderr.clone()
                } else {
                    format!("Exit code: {}", output.exit_code)
                };

                if attempt < MAX_RETRIES {
                    info!(task_id, attempt, error = %error_msg, "Command failed, asking LLM to fix");

                    let retry_prompt = RETRY_PROMPT
                        .replace("{ERROR}", &error_msg)
                        .replace("{COMMAND}", &current_script);

                    let fix_prompt = format!(
                        "Original task: \"{}\"\n\n{}",
                        original_task, retry_prompt
                    );

                    match gateway.complete_with_system(&fix_prompt, Some(PC_MASTER_PROMPT), settings).await {
                        Ok(fix_response) => {
                            if let Some(fix_json) = extract_json(fix_response.content.trim()) {
                                if let Some(cmds) = fix_json["commands"].as_array() {
                                    let new_cmds: Vec<&str> = cmds.iter().filter_map(|v| v.as_str()).collect();
                                    if !new_cmds.is_empty() {
                                        current_script = new_cmds.join("; ");
                                        info!(task_id, attempt, new_script = %current_script, "LLM provided fixed command");
                                        continue;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!(task_id, error = %e, "LLM retry failed");
                        }
                    }
                }

                // All retries exhausted
                return (false, format!("Error: {}", error_msg));
            }
            Err(e) => {
                let err = format!("Execution error: {}", e);
                if attempt >= MAX_RETRIES {
                    return (false, err);
                }
                warn!(task_id, attempt, error = %err, "Execution failed, retrying");
            }
        }
    }

    (false, "Failed after all retries".to_string())
}

// ═══════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════

fn extract_json(text: &str) -> Option<serde_json::Value> {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        return Some(v);
    }
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text[start..=end]) {
                return Some(v);
            }
        }
    }
    None
}

async fn take_screenshot(dir: &Path) -> Option<std::path::PathBuf> {
    let sd = dir.to_path_buf();
    tokio::task::spawn_blocking(move || {
        capture::capture_full_screen()
            .ok()
            .and_then(|data| capture::save_screenshot(&data, &sd).ok())
    }).await.ok().flatten()
}

fn update_task_status(db_path: &Path, task_id: &str, status: &str) {
    if let Ok(db) = Database::new(db_path) {
        let _ = db.update_task_status(task_id, status);
    }
}

fn save_task_output(db_path: &Path, task_id: &str, output: &str) {
    if let Ok(db) = Database::new(db_path) {
        let _ = db.update_task_output(task_id, output);
    }
}

fn save_step(
    db_path: &Path, task_id: &str, step_number: u32,
    action: &AgentAction, screenshot_path: &Path, result: &ExecutionResult,
) {
    if let Ok(db) = Database::new(db_path) {
        let action_type = match action {
            AgentAction::Click { .. } => "click",
            AgentAction::DoubleClick { .. } => "double_click",
            AgentAction::RightClick { .. } => "right_click",
            AgentAction::Type { .. } => "type",
            AgentAction::KeyCombo { .. } => "key_combo",
            AgentAction::Scroll { .. } => "scroll",
            AgentAction::RunCommand { .. } => "run_command",
            AgentAction::Wait { .. } => "wait",
            AgentAction::Screenshot => "screenshot",
            AgentAction::TaskComplete { .. } => "task_complete",
        };
        let description = serde_json::to_string(action).unwrap_or_default();
        let exec_method = match result.method {
            ExecutionMethod::Api => "api",
            ExecutionMethod::Terminal => "terminal",
            ExecutionMethod::Screen => "screen",
        };
        let _ = db.insert_task_step(
            task_id, step_number, action_type, &description,
            &screenshot_path.to_string_lossy(), exec_method,
            result.success, result.duration_ms,
        );
    }
}
