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
use tracing::{info, warn, error};

const MAX_STEPS: u32 = 30;

// ═══════════════════════════════════════════════════════════════════
// THE MASTER SYSTEM PROMPT — this is the brain of the entire agent
// ═══════════════════════════════════════════════════════════════════
const PC_MASTER_PROMPT: &str = r#"You are AgentOS, an AI that CONTROLS a Windows PC. You receive a task and you MUST execute it completely.

You have TWO modes of operation. Respond with a JSON object in ONE of these formats:

═══ MODE 1: COMMAND (for anything you can do via PowerShell) ═══
{"mode": "command", "commands": ["command1", "command2"], "explanation": "what these do"}

═══ MODE 2: SCREEN (when you need to interact with GUI visually) ═══
{"mode": "screen", "reason": "why I need to see the screen"}

═══ MODE 3: DONE (when the task is fully completed) ═══
{"mode": "done", "summary": "what was accomplished", "output": "any relevant data to show the user"}

═══ MODE 4: NEED_INFO (when you need clarification) ═══
{"mode": "need_info", "question": "what you need to know"}

═══ MODE 5: CHAT (for questions that don't need PC action) ═══
{"mode": "chat", "response": "your conversational answer here"}

Use CHAT mode ONLY for abstract questions like "what is python" or "explain machine learning".
For ANYTHING that involves checking, reading, showing, listing, or doing something on the PC, use COMMAND mode.
"cuanto espacio tengo" → COMMAND (run Get-PSDrive)
"que archivos hay" → COMMAND (run Get-ChildItem)
"que hora es" → COMMAND (run Get-Date)
"que programas tengo instalados" → COMMAND (run Get-Package)

RULES:
1. ALWAYS respond with valid JSON. Nothing else.
2. Prefer MODE 1 (commands) — it's faster and more reliable than screen interaction.
3. Use PowerShell for EVERYTHING you can:
   - Open apps: Start-Process 'app.exe'
   - Open folders: Start-Process explorer.exe 'C:\path'
   - Open URLs: Start-Process 'https://url.com'
   - Create files: Set-Content -Path 'file.txt' -Value 'content'
   - Edit files: (Get-Content file.txt) -replace 'old','new' | Set-Content file.txt
   - Read files: Get-Content 'file.txt'
   - Read PDFs: (requires text extraction approach)
   - Move files: Move-Item -Path 'source' -Destination 'dest'
   - Copy files: Copy-Item -Path 'source' -Destination 'dest'
   - Delete files: Remove-Item 'path' (ONLY if user explicitly asks)
   - Rename: Rename-Item -Path 'old' -NewName 'new'
   - Search files: Get-ChildItem -Path 'dir' -Filter '*.ext' -Recurse
   - System info: Get-ComputerInfo, Get-PSDrive, systeminfo
   - Network: Test-NetConnection, Get-NetAdapter, ipconfig
   - Processes: Get-Process, Stop-Process -Name 'x'
   - Services: Get-Service, Start-Service, Stop-Service
   - Registry: Get-ItemProperty 'HKLM:\path'
   - Installed apps: Get-Package or Get-WmiObject Win32_Product
   - Screen brightness: (Get-WmiObject -Namespace root/wmi -Class WmiMonitorBrightnessMethods).WmiSetBrightness(1,80)
   - Volume: use nircmd or PowerShell audio COM
   - Clipboard: Get-Clipboard, Set-Clipboard
   - Compress: Compress-Archive -Path 'source' -DestinationPath 'dest.zip'
   - Extract: Expand-Archive -Path 'file.zip' -DestinationPath 'dest'
   - Download: Invoke-WebRequest -Uri 'url' -OutFile 'file'
   - User env vars: [Environment]::GetEnvironmentVariable('VAR','User')
   - Set env vars: [Environment]::SetEnvironmentVariable('VAR','VALUE','User')
   - Scheduled tasks: Register-ScheduledTask, Get-ScheduledTask
   - Windows settings: start ms-settings:display, ms-settings:network, ms-settings:bluetooth, etc.
   - Dark mode: Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize' -Name 'AppsUseLightTheme' -Value 0

4. Use $env:USERPROFILE for the user's home folder
5. You can chain multiple commands to accomplish complex tasks
6. For multi-step tasks, return ALL commands needed in the array
7. For reading documents, extract text and return it as output
8. NEVER format a disk, delete system files, or run destructive operations unless explicitly asked
9. If a task requires seeing the screen (clicking buttons in an app, reading visual content), use MODE 2
10. After executing commands, if the user needs to see results, include output-producing commands

EXAMPLES:

User: "abre la carpeta descargas"
{"mode": "command", "commands": ["Start-Process explorer.exe \"$env:USERPROFILE\\Downloads\""], "explanation": "Opening Downloads folder in Explorer"}

User: "dime que archivos hay en mi escritorio"
{"mode": "command", "commands": ["Get-ChildItem \"$env:USERPROFILE\\Desktop\" | Format-Table Name, Length, LastWriteTime -AutoSize"], "explanation": "Listing Desktop files"}

User: "crea un documento de texto en el escritorio que diga hola mundo"
{"mode": "command", "commands": ["Set-Content -Path \"$env:USERPROFILE\\Desktop\\hola.txt\" -Value 'Hola Mundo' -Encoding UTF8", "Write-Output 'File created: hola.txt on Desktop'"], "explanation": "Creating text file on Desktop"}

User: "organiza mis descargas por tipo de archivo"
{"mode": "command", "commands": ["$dl = \"$env:USERPROFILE\\Downloads\"", "$types = @{Images=@('.jpg','.jpeg','.png','.gif','.bmp','.svg','.webp');Documents=@('.pdf','.doc','.docx','.xls','.xlsx','.ppt','.pptx','.txt','.csv');Videos=@('.mp4','.avi','.mkv','.mov','.wmv');Audio=@('.mp3','.wav','.flac','.aac','.ogg');Archives=@('.zip','.rar','.7z','.tar','.gz');Code=@('.py','.js','.ts','.html','.css','.rs','.java','.cpp')}", "foreach($cat in $types.Keys){$dir=\"$dl\\$cat\";if(!(Test-Path $dir)){New-Item -ItemType Directory -Path $dir -Force|Out-Null};foreach($ext in $types[$cat]){Get-ChildItem \"$dl\\*$ext\" -File -ErrorAction SilentlyContinue|Move-Item -Destination $dir -Force}}", "Get-ChildItem $dl -Directory | ForEach-Object { $count = (Get-ChildItem $_.FullName -File).Count; Write-Output \"$($_.Name): $count files\" }"], "explanation": "Creating folders by file type and moving files into them"}

User: "que procesos estan usando mas memoria"
{"mode": "command", "commands": ["Get-Process | Sort-Object WorkingSet64 -Descending | Select-Object -First 15 Name, @{N='Memory(MB)';E={[math]::Round($_.WorkingSet64/1MB,1)}}, @{N='CPU(s)';E={[math]::Round($_.CPU,1)}} | Format-Table -AutoSize"], "explanation": "Showing top 15 processes by memory usage"}

User: "pon el modo oscuro"
{"mode": "command", "commands": ["Set-ItemProperty -Path 'HKCU:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize' -Name 'AppsUseLightTheme' -Value 0", "Set-ItemProperty -Path 'HKCU:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize' -Name 'SystemUsesLightTheme' -Value 0", "Write-Output 'Dark mode enabled. Some apps may need restart to apply.'"], "explanation": "Enabling dark mode via registry"}

User: "lee el archivo readme.md de mi escritorio"
{"mode": "command", "commands": ["if(Test-Path \"$env:USERPROFILE\\Desktop\\readme.md\"){Get-Content \"$env:USERPROFILE\\Desktop\\readme.md\" -Raw}else{Write-Output 'File not found: readme.md on Desktop'}"], "explanation": "Reading file content"}

User: "descarga esta imagen https://example.com/photo.jpg y guardala en el escritorio"
{"mode": "command", "commands": ["Invoke-WebRequest -Uri 'https://example.com/photo.jpg' -OutFile \"$env:USERPROFILE\\Desktop\\photo.jpg\"", "Write-Output 'Downloaded to Desktop/photo.jpg'"], "explanation": "Downloading file from URL"}

User: "abre la configuracion de pantalla"
{"mode": "command", "commands": ["Start-Process ms-settings:display"], "explanation": "Opening Windows display settings"}

User: "renombra todos los archivos .jpeg a .jpg en descargas"
{"mode": "command", "commands": ["$count = 0; Get-ChildItem \"$env:USERPROFILE\\Downloads\\*.jpeg\" | ForEach-Object { $newName = $_.FullName -replace '\\.jpeg$','.jpg'; Rename-Item $_.FullName $newName; $count++ }; Write-Output \"Renamed $count files from .jpeg to .jpg\""], "explanation": "Batch renaming files"}

User: "comprime la carpeta proyectos del escritorio"
{"mode": "command", "commands": ["$src = \"$env:USERPROFILE\\Desktop\\proyectos\"; $dst = \"$env:USERPROFILE\\Desktop\\proyectos.zip\"; if(Test-Path $src){Compress-Archive -Path $src -DestinationPath $dst -Force; Write-Output \"Compressed to proyectos.zip\"}else{Write-Output 'Folder not found'}"], "explanation": "Compressing folder to zip"}

User: "necesito hacer click en un boton de una app"
{"mode": "screen", "reason": "Need to visually locate and click a button in an application UI"}
"#;

// ═══════════════════════════════════════════════════════════════════
// MAIN ENGINE
// ═══════════════════════════════════════════════════════════════════

/// Run a complete PC control task — the definitive engine
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

    // Step 0: Ask the LLM what to do
    let _ = app_handle.emit("agent:step_started", serde_json::json!({
        "task_id": task_id, "step_number": 0,
    }));

    let plan = gateway
        .complete_with_system(description, Some(PC_MASTER_PROMPT), settings)
        .await
        .map_err(|e| {
            update_task_status(db_path, task_id, "failed");
            format!("LLM failed: {}", e)
        })?;

    let plan_text = plan.content.trim().to_string();
    info!(task_id, plan = %plan_text, "LLM plan received");

    // Parse the JSON response
    let plan_json = match extract_json(&plan_text) {
        Some(json) => json,
        None => {
            // LLM didn't return JSON — treat the raw response as a chat answer
            warn!(task_id, "LLM didn't return JSON, treating as chat response");
            let output = plan_text.clone();
            save_task_output(db_path, task_id, &output);
            update_task_status(db_path, task_id, "completed");

            let _ = app_handle.emit("agent:task_completed", serde_json::json!({
                "task_id": task_id, "success": true, "output": output,
            }));

            return Ok(TaskExecutionResult {
                task_id: task_id.to_string(),
                success: true,
                steps: vec![],
                total_cost: 0.0,
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
    };

    let mode = plan_json["mode"].as_str().unwrap_or("command");

    match mode {
        "command" => {
            // ── COMMAND MODE: Execute PowerShell commands ──
            let commands = plan_json["commands"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();
            let explanation = plan_json["explanation"].as_str().unwrap_or("");

            if commands.is_empty() {
                // If no commands but explanation exists, show it
                if !explanation.is_empty() {
                    save_task_output(db_path, task_id, explanation);
                    update_task_status(db_path, task_id, "completed");
                    accumulated_output = explanation.to_string();
                } else {
                    save_task_output(db_path, task_id, "No commands generated");
                    update_task_status(db_path, task_id, "failed");
                }
                // Skip to end
            } else {

            info!(task_id, count = commands.len(), "Executing {} commands", commands.len());

            // Join all commands into one PowerShell script
            let full_script = commands.join("; ");

            let _ = app_handle.emit("agent:step_started", serde_json::json!({
                "task_id": task_id,
                "step_number": 1,
                "description": explanation,
            }));

            // Determine timeout — GUI launches are fast, data commands may be slow
            let is_gui = full_script.to_lowercase().contains("start-process");
            let timeout = if is_gui { 30 } else { settings.cli_timeout };

            // Execute
            let exec_result = hands::cli::run_powershell(&full_script, timeout).await;

            let (success, output) = match exec_result {
                Ok(cmd_output) => {
                    let out = if !cmd_output.stdout.trim().is_empty() {
                        cmd_output.stdout.clone()
                    } else if !cmd_output.stderr.trim().is_empty() {
                        // Some commands write to stderr even on success
                        if cmd_output.exit_code == 0 {
                            cmd_output.stderr.clone()
                        } else {
                            format!("Error: {}", cmd_output.stderr)
                        }
                    } else {
                        explanation.to_string()
                    };
                    (cmd_output.exit_code == 0, out)
                }
                Err(e) => (false, format!("Execution failed: {}", e)),
            };

            info!(task_id, success, "Commands executed");

            // Screenshot after execution
            if is_gui {
                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            }

            let screenshot_path = take_screenshot(screenshots_dir).await;
            let sp = screenshot_path.as_ref().map(|p| p.to_string_lossy().to_string());

            // Save step
            let action = AgentAction::RunCommand {
                command: full_script.clone(),
                shell: ShellType::PowerShell,
            };
            let result = ExecutionResult {
                method: ExecutionMethod::Terminal,
                success,
                output: Some(output.clone()),
                screenshot_path: sp.clone(),
                duration_ms: start.elapsed().as_millis() as u64,
            };
            save_step(db_path, task_id, 1, &action, &screenshot_path.unwrap_or_default(), &result);

            let _ = app_handle.emit("agent:step_completed", serde_json::json!({
                "task_id": task_id, "step_number": 1, "success": success,
                "output": output, "command": full_script,
            }));

            step_history.push(StepRecord {
                step_number: 1,
                action: AgentAction::TaskComplete { summary: output.clone() },
                result,
                screenshot_path: sp,
            });

            accumulated_output = output;
            update_task_status(db_path, task_id, if success { "completed" } else { "failed" });
            } // end else (commands not empty)
        }

        "screen" => {
            // ── SCREEN MODE: Vision-guided autonomous loop ──
            let reason = plan_json["reason"].as_str().unwrap_or("Complex UI task");
            info!(task_id, reason, "Entering vision-guided mode");

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
                })
                .await
                .map_err(|e| e.to_string())??;

                let (screenshot_path, screenshot_b64) = screenshot;

                let _ = app_handle.emit("agent:step_started", serde_json::json!({
                    "task_id": task_id, "step_number": step_number,
                }));

                let action = vision::plan_next_action(
                    &screenshot_b64, description, &step_history, settings, &gateway,
                ).await;

                let action = match action {
                    Ok(a) => a,
                    Err(e) => {
                        warn!(task_id, step_number, error = %e, "Vision failed");
                        update_task_status(db_path, task_id, "failed");
                        return Err(format!("Vision LLM failed: {}", e));
                    }
                };

                if matches!(action, AgentAction::TaskComplete { .. }) {
                    if let AgentAction::TaskComplete { ref summary } = action {
                        accumulated_output = summary.clone();
                    }
                    let result = ExecutionResult {
                        method: ExecutionMethod::Screen, success: true,
                        output: Some(accumulated_output.clone()),
                        screenshot_path: Some(screenshot_path.to_string_lossy().to_string()),
                        duration_ms: 0,
                    };
                    step_history.push(StepRecord {
                        step_number, action, result,
                        screenshot_path: Some(screenshot_path.to_string_lossy().to_string()),
                    });
                    update_task_status(db_path, task_id, "completed");
                    break;
                }

                let exec_result = executor::execute(&action, settings.cli_timeout, kill_switch).await;
                let result = match exec_result {
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
                update_task_status(db_path, task_id, "failed");
            }
        }

        "done" => {
            accumulated_output = plan_json["output"].as_str()
                .or(plan_json["summary"].as_str())
                .unwrap_or("Task completed")
                .to_string();
            update_task_status(db_path, task_id, "completed");
        }

        "need_info" => {
            let question = plan_json["question"].as_str().unwrap_or("I need more information");
            accumulated_output = question.to_string();
            update_task_status(db_path, task_id, "completed");
        }

        "chat" => {
            accumulated_output = plan_json["response"].as_str()
                .unwrap_or("I'm not sure how to help with that.")
                .to_string();
            update_task_status(db_path, task_id, "completed");
        }

        _ => {
            update_task_status(db_path, task_id, "failed");
            return Err(format!("Unknown mode: {}", mode));
        }
    }

    // Save output to DB so polling can find it
    if !accumulated_output.is_empty() {
        save_task_output(db_path, task_id, &accumulated_output);
    }

    // Emit final completion
    let duration_ms = start.elapsed().as_millis() as u64;
    let success = step_history.last().map(|s| s.result.success).unwrap_or(true);

    let _ = app_handle.emit("agent:task_completed", serde_json::json!({
        "task_id": task_id,
        "success": success,
        "output": accumulated_output,
        "steps": step_history.len(),
        "duration_ms": duration_ms,
    }));

    Ok(TaskExecutionResult {
        task_id: task_id.to_string(),
        success,
        steps: step_history,
        total_cost: 0.0,
        duration_ms,
    })
}

// ═══════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════

fn extract_json(text: &str) -> Option<serde_json::Value> {
    // Try parsing the whole text first
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        return Some(v);
    }
    // Find JSON object in text (LLM might wrap it in markdown)
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
