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
use tauri::{Emitter, Manager};
use tracing::{info, warn};

/// Scale coordinates from LLM image space to real screen space.
/// Uses capture_w/capture_h (physical pixel dimensions from GDI BitBlt)
/// instead of GetSystemMetrics which returns logical pixels on HiDPI displays.
fn scale_action_coords(action: AgentAction, img_w: u32, img_h: u32, capture_w: u32, capture_h: u32) -> AgentAction {
    if capture_w == 0 || capture_h == 0 || img_w == 0 || img_h == 0 {
        return action;
    }

    let scale = |x: i32, y: i32| -> (i32, i32) {
        let real_x = (x as f64 * capture_w as f64 / img_w as f64) as i32;
        let real_y = (y as f64 * capture_h as f64 / img_h as f64) as i32;
        (real_x, real_y)
    };

    match action {
        AgentAction::Click { x, y } => {
            let (rx, ry) = scale(x, y);
            AgentAction::Click { x: rx, y: ry }
        }
        AgentAction::DoubleClick { x, y } => {
            let (rx, ry) = scale(x, y);
            AgentAction::DoubleClick { x: rx, y: ry }
        }
        AgentAction::RightClick { x, y } => {
            let (rx, ry) = scale(x, y);
            AgentAction::RightClick { x: rx, y: ry }
        }
        AgentAction::Scroll { x, y, delta } => {
            let (rx, ry) = scale(x, y);
            AgentAction::Scroll { x: rx, y: ry, delta }
        }
        other => other,
    }
}

const MAX_TURNS: u32 = 10;
const MAX_RETRIES: u32 = 2;
const MAX_BROWSER_OPENS: u32 = 3;
const BROWSER_OPEN_DELAY_MS: u64 = 2000;

const SYSTEM_PROMPT: &str = r#"You are AgentOS, an AI agent that CONTROLS a Windows 11 PC. You execute tasks step by step until they are FULLY completed.

You work in a LOOP. Each turn you receive:
- The original task
- Results from previous steps (what commands ran, what output they produced, any errors)
- You decide the NEXT action

Respond with ONE JSON object per turn:

{"mode":"command","commands":["ps command"],"explanation":"what this does"}
{"mode":"multi","steps":[{"commands":["cmd1"],"explanation":"step 1"},{"commands":["cmd2"],"explanation":"step 2"}]}
{"mode":"screen","reason":"why I need to see the screen"}
{"mode":"done","summary":"what was accomplished","output":"final data for user"}
{"mode":"chat","response":"conversational answer"}
{"mode":"need_info","question":"what I need to know"}

KEY BEHAVIORS:
1. For COMPLEX tasks, use "multi" mode — it runs steps sequentially, each step sees output of previous
2. For SIMPLE tasks, use "command" mode — single execution
3. After a command runs, you'll see its output. If it needs follow-up, you get another turn
4. Use "done" when the ENTIRE task is finished — include useful output for the user
5. If a previous step failed, analyze the error and try a different approach
6. You have MEMORY of all previous steps in this task — use it

POWERSHELL RULES:
- Single quotes for literals: 'text'
- Double quotes for variable expansion: "$env:USERPROFILE\path"
- NEVER use $var:\ syntax — use Join-Path or "$($var)path"
- Use -ErrorAction SilentlyContinue for non-critical ops
- End scripts with Write-Output for visible results
- try/catch for robustness
- Start-Process for GUI apps (non-blocking)
- For Windows Settings: Start-Process 'ms-settings:page'
- For drives: Get-PSDrive -PSProvider FileSystem
- For hidden files: Get-ChildItem -Force -Hidden
- Format-Table -AutoSize for readable output

CAPABILITIES:
Files: Create, read, edit, move, copy, rename, delete, search (Get-ChildItem, Select-String)
Apps: Open any app, folder, URL, Windows settings
System: Disk info, RAM, CPU, processes, services, network, battery, OS info
Config: Dark mode, wallpaper, env vars, registry, scheduled tasks
Archives: Compress-Archive, Expand-Archive
Network: Invoke-WebRequest, Test-NetConnection, Get-NetAdapter
Data: Import-Csv, ConvertFrom-Json, [xml], Excel COM automation
Dev: git, docker, python, node, npm, cargo (if installed)
Office: Word/Excel/PowerPoint via COM automation
PDF: Text extraction approaches
Web search/scraping: Use Invoke-WebRequest to fetch page content directly (PREFERRED over opening browser)

CRITICAL — WEB SEARCHES AND PRICE LOOKUPS:
When the user asks to search the web, look up prices, or find information online:
1. ALWAYS use Invoke-WebRequest to fetch the page directly — DO NOT open browser windows
2. Parse the HTML to extract the information needed
3. NEVER open multiple browser tabs/windows in a loop
4. If you must open a browser, open it ONCE and use screen mode to navigate

Example for web search:
"busca en mercado libre el precio de una PS5"
{"mode":"command","commands":["$url = 'https://listado.mercadolibre.com.uy/playstation-5#D[A:playstation%205]'; $r = Invoke-WebRequest -Uri $url -UseBasicParsing; $items = [regex]::Matches($r.Content, 'class=\"poly-price__current\">.*?<span.*?>(.*?)</span>.*?class=\"poly-component__title\">(.*?)</a>', 'Singleline') | Select-Object -First 4; if($items.Count -eq 0){ Write-Output 'Could not parse results. Try: Start-Process $url' } else { $i=1; foreach($m in $items){ Write-Output \"$i. $($m.Groups[2].Value.Trim()) - $($m.Groups[1].Value.Trim())\"; $i++ } }"],"explanation":"Fetching MercadoLibre results via web scraping"}

NEVER do this (opens infinite browser windows):
BAD: Running Start-Process 'https://...' inside a loop or multi-step
BAD: Using screen mode to repeatedly open new URLs

IMPORTANT — MIXING COMMAND AND SCREEN MODES:
For tasks that need BOTH terminal commands AND visual interaction (like downloading + installing software), use "multi" with a special "screen_after" flag on the last step that needs screen interaction:

{"mode":"command_then_screen","commands":["PowerShell commands to download/prepare"],"screen_task":"what to do visually after commands run","explanation":"description"}

This will: 1) Run the PowerShell commands 2) Switch to SCREEN mode to handle the visual part (installer wizard, browser interaction, etc.)

DOWNLOAD + INSTALL FLOW:
1. Use Invoke-WebRequest or Start-Process with a URL to download
2. For .exe/.msi installers: use command_then_screen to run the installer, then vision handles the wizard
3. For .msi silent installs: msiexec /i 'file.msi' /quiet /norestart (no screen needed)
4. For winget (Windows Package Manager): winget install 'AppName' --accept-source-agreements --accept-package-agreements
5. For chocolatey: choco install appname -y (if installed)
6. For browser downloads: use Start-Process with the download URL, then screen mode to handle browser dialog

EXAMPLES:

"descarga e instala VLC"
{"mode":"command","commands":["winget install VideoLAN.VLC --accept-source-agreements --accept-package-agreements; Write-Output 'VLC installation complete'"],"explanation":"Installing VLC via winget (silent)"}

"descarga e instala un juego de steam"
{"mode":"command_then_screen","commands":["Start-Process 'https://store.steampowered.com'"],"screen_task":"Navigate Steam website, find a free game, click Install/Play, handle any dialogs","explanation":"Opening Steam store, then using screen to browse and install"}

"busca en google como hacer X y seguí los pasos"
{"mode":"command_then_screen","commands":["Start-Process 'https://www.google.com/search?q=how+to+do+X'"],"screen_task":"Read search results, click the best one, follow the instructions shown on the page","explanation":"Google search then follow instructions visually"}

"descarga firefox e instalalo"
{"mode":"multi","steps":[
  {"commands":["Invoke-WebRequest -Uri 'https://download.mozilla.org/?product=firefox-latest&os=win64&lang=en-US' -OutFile \"$env:TEMP\\firefox_installer.exe\"; Write-Output 'Firefox downloaded'"],"explanation":"Download Firefox installer"},
  {"commands":["Start-Process \"$env:TEMP\\firefox_installer.exe\" -ArgumentList '/S' -Wait; Write-Output 'Firefox installed silently'"],"explanation":"Run silent install"}
],"explanation":"Download and silently install Firefox"}

"instala notepad++ desde la web"
{"mode":"command_then_screen","commands":["Start-Process 'https://notepad-plus-plus.org/downloads/'"],"screen_task":"Click the download link for the latest version, handle browser download dialog, then run the installer and click through the wizard (Next, I Agree, Next, Install, Finish)","explanation":"Download from website then install via wizard"}

"abre youtube y busca videos de programacion"
{"mode":"command_then_screen","commands":["Start-Process 'https://www.youtube.com'"],"screen_task":"Wait for YouTube to load, click the search bar, type 'programacion tutorial', press Enter","explanation":"Open YouTube then search"}

GUI APP INTERACTION — CRITICAL:
When a task requires CLICKING BUTTONS in a GUI app (calculator, notepad save dialog, settings, etc.), you MUST use "command_then_screen" — NOT just "command". Opening the app alone does NOT complete the task.

"abre la calculadora y calcula 125 + 375"
{"mode":"command_then_screen","commands":["Start-Process calc.exe; Start-Sleep -Seconds 2"],"screen_task":"Click buttons: 1, 2, 5, then +, then 3, 7, 5, then = to calculate 125+375. Read the result from the display.","explanation":"Open calculator then click buttons to compute"}

"abre el bloc de notas, escribe 'Hola' y guardalo en el escritorio como test.txt"
{"mode":"command_then_screen","commands":["Start-Process notepad.exe; Start-Sleep -Seconds 1"],"screen_task":"Type 'Hola' in the notepad window. Then press Ctrl+S to save. In the Save As dialog, navigate to Desktop, type 'test.txt' as filename, and click Save.","explanation":"Open notepad then type and save via GUI"}

"abre el explorador y navega a mis documentos"
{"mode":"command_then_screen","commands":["Start-Process explorer.exe; Start-Sleep -Seconds 2"],"screen_task":"In the Explorer window, click on 'Documents' or 'Documentos' in the left sidebar. Report what files are visible.","explanation":"Open explorer then navigate visually"}

"cambia el fondo de pantalla a negro"
{"mode":"command","commands":["Set-ItemProperty -Path 'HKCU:\\Control Panel\\Desktop' -Name Wallpaper -Value ''; Set-ItemProperty -Path 'HKCU:\\Control Panel\\Colors' -Name Background -Value '0 0 0'; Add-Type -TypeDefinition 'using System;using System.Runtime.InteropServices;public class W{[DllImport(\"user32.dll\",CharSet=CharSet.Auto)]public static extern int SystemParametersInfo(int a,int b,string c,int d);}'; [W]::SystemParametersInfo(20,0,'',3); Write-Output 'Wallpaper changed to solid black'"],"explanation":"Change wallpaper to solid black via registry + SystemParametersInfo"}

RULE: If the task involves clicking buttons, typing text into a GUI app, navigating menus, or reading visual information from an app window — you MUST use "command_then_screen" or "screen" mode. Using only "command" mode for these tasks is WRONG.

MULTI-STEP EXAMPLE:
"analiza mi disco, encuentra archivos grandes, y crea un reporte"
{"mode":"multi","steps":[
  {"commands":["Get-PSDrive -PSProvider FileSystem | Where-Object {$_.Used -gt 0} | Select-Object Name, @{N='Used(GB)';E={[math]::Round($_.Used/1GB,2)}}, @{N='Free(GB)';E={[math]::Round($_.Free/1GB,2)}} | Format-Table -AutoSize"],"explanation":"Check disk usage"},
  {"commands":["Get-ChildItem 'C:\\Users' -Recurse -File -ErrorAction SilentlyContinue | Sort-Object Length -Descending | Select-Object -First 10 FullName, @{N='MB';E={[math]::Round($_.Length/1MB,2)}} | Format-Table -AutoSize"],"explanation":"Find largest files"},
  {"commands":["$r = \"Disk Report - $(Get-Date)`n\"; Get-PSDrive -PSProvider FileSystem | Where-Object {$_.Used -gt 0} | ForEach-Object { $r += \"$($_.Name): $([math]::Round($_.Free/1GB,2))GB free`n\" }; $r += \"`nLargest files:`n\"; Get-ChildItem 'C:\\Users' -Recurse -File -ErrorAction SilentlyContinue | Sort-Object Length -Descending | Select-Object -First 10 | ForEach-Object { $r += \"$([math]::Round($_.Length/1MB,2))MB - $($_.Name)`n\" }; Set-Content \"$env:USERPROFILE\\Desktop\\disk_report.txt\" $r; Write-Output $r"],"explanation":"Generate report"}
],"explanation":"Full disk analysis"}
"#;

const RETRY_PROMPT: &str = r#"The command FAILED with this error:
```
{ERROR}
```
Original command: {COMMAND}

Fix it. Common issues: don't use $var:\ syntax, use Join-Path, add -ErrorAction SilentlyContinue, wrap in try/catch.
Respond with: {"mode":"command","commands":[...],"explanation":"..."}
"#;

const FOLLOWUP_PROMPT: &str = r#"PREVIOUS STEP RESULT:
Command: {COMMAND}
Success: {SUCCESS}
Output:
```
{OUTPUT}
```

The original task was: "{TASK}"
{REMAINING}

Based on this result, what should happen next? If the task is complete, use "done" mode with the final output. If more steps are needed, use "command" mode.
"#;

// ═══════════════════════════════════════════════════════════════════
// MAIN ENGINE — MULTI-TURN EXECUTION LOOP
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
    let mut conversation: Vec<(String, String)> = Vec::new(); // (role, content) history
    let mut browser_opens: u32 = 0;

    info!(task_id, description, "Starting PC task");

    emit(app_handle, "agent:step_started", task_id, 0, "Planning...");

    // Initial LLM call — force Standard tier (sonnet/gpt-4o) for PC control
    // Cheap models (haiku, flash) can't follow the complex agent system prompt
    let plan = gateway.complete_as_agent(description, SYSTEM_PROMPT, settings)
        .await
        .map_err(|e| { fail(db_path, task_id, &e); e })?;

    let mut current_response = plan.content.trim().to_string();
    conversation.push(("user".into(), description.to_string()));
    conversation.push(("assistant".into(), current_response.clone()));

    // Multi-turn execution loop
    for turn in 0..MAX_TURNS {
        if kill_switch.load(Ordering::Relaxed) {
            update_task_status(db_path, task_id, "killed");
            return Err("Kill switch activated".to_string());
        }

        let plan_json = match extract_json(&current_response) {
            Some(j) => j,
            None => {
                // Raw text response — treat as chat
                accumulated_output = current_response.clone();
                update_task_status(db_path, task_id, "completed");
                break;
            }
        };

        let mode = plan_json["mode"].as_str().unwrap_or("command");
        info!(task_id, turn, mode, "Processing turn");

        match mode {
            "multi" => {
                // Multi-step: execute each step sequentially, feeding output forward
                let steps = plan_json["steps"].as_array().cloned().unwrap_or_default();
                let mut all_outputs = Vec::new();

                for (i, step_def) in steps.iter().enumerate() {
                    if kill_switch.load(Ordering::Relaxed) { break; }

                    let cmds = step_def["commands"].as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                        .unwrap_or_default();
                    let expl = step_def["explanation"].as_str().unwrap_or("");

                    if cmds.is_empty() { continue; }

                    let script = cmds.join("; ");
                    let step_num = (turn * 10 + i as u32) + 1;

                    // Browser spam guard
                    let opens = count_browser_opens(&script);
                    if opens > 0 {
                        browser_opens += opens;
                        if browser_opens > MAX_BROWSER_OPENS {
                            warn!(task_id, browser_opens, "Browser open limit reached, aborting to prevent spam loop");
                            accumulated_output = format!("Stopped: opened {} browser windows (limit is {}). Use Invoke-WebRequest instead of opening browser windows.", browser_opens, MAX_BROWSER_OPENS);
                            update_task_status(db_path, task_id, "failed");
                            break;
                        }
                        info!(task_id, browser_opens, "Browser window opened ({}/{})", browser_opens, MAX_BROWSER_OPENS);
                    }

                    emit(app_handle, "agent:step_started", task_id, step_num, expl);
                    info!(task_id, step = step_num, script = %script, "Multi-step executing");

                    let is_gui = script.to_lowercase().contains("start-process");
                    let timeout = if is_gui { 30 } else { settings.cli_timeout };

                    let (success, output) = execute_with_retry(
                        &script, timeout, description, &gateway, settings, task_id,
                    ).await;

                    if is_gui { tokio::time::sleep(std::time::Duration::from_millis(1500)).await; }

                    let sp = take_screenshot(screenshots_dir).await;
                    let sp_str = sp.as_ref().map(|p| p.to_string_lossy().to_string());

                    let action = AgentAction::RunCommand { command: script.clone(), shell: ShellType::PowerShell };
                    let result = ExecutionResult {
                        method: ExecutionMethod::Terminal, success,
                        output: Some(output.clone()), screenshot_path: sp_str.clone(),
                        duration_ms: start.elapsed().as_millis() as u64,
                    };
                    save_step(db_path, task_id, step_num, &action, &sp.unwrap_or_default(), &result);

                    emit(app_handle, "agent:step_completed", task_id, step_num, &format!("success={}", success));

                    step_history.push(StepRecord {
                        step_number: step_num, action, result, screenshot_path: sp_str,
                    });

                    all_outputs.push(format!("Step {} ({}): {}", i + 1, expl, if success { &output } else { "FAILED" }));

                    if !success {
                        // Step failed — ask LLM to handle
                        let followup = format!(
                            "Step {} failed.\nCommand: {}\nError: {}\n\nOriginal task: \"{}\"\nShould I retry differently or skip this step?",
                            i + 1, script, output, description
                        );
                        conversation.push(("user".into(), followup.clone()));

                        if let Ok(fix) = gateway.complete_as_agent(&followup, SYSTEM_PROMPT, settings).await {
                            current_response = fix.content.trim().to_string();
                            conversation.push(("assistant".into(), current_response.clone()));
                            // Will be processed in the next turn
                        }
                        break;
                    }
                }

                accumulated_output = all_outputs.join("\n\n");

                // If all steps completed successfully, we're done
                if step_history.last().map(|s| s.result.success).unwrap_or(false) {
                    update_task_status(db_path, task_id, "completed");
                    break;
                }
            }

            "command" => {
                let cmds = plan_json["commands"].as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                    .unwrap_or_default();
                let expl = plan_json["explanation"].as_str().unwrap_or("");

                if cmds.is_empty() {
                    accumulated_output = if !expl.is_empty() { expl.to_string() } else { "No commands".to_string() };
                    update_task_status(db_path, task_id, "completed");
                    break;
                }

                let script = cmds.join("; ");
                let step_num = (turn + 1) as u32;

                // Browser spam guard
                let opens = count_browser_opens(&script);
                if opens > 0 {
                    browser_opens += opens;
                    if browser_opens > MAX_BROWSER_OPENS {
                        warn!(task_id, browser_opens, "Browser open limit reached in command mode");
                        accumulated_output = format!("Stopped: opened {} browser windows (limit is {}). Use Invoke-WebRequest instead.", browser_opens, MAX_BROWSER_OPENS);
                        update_task_status(db_path, task_id, "failed");
                        break;
                    }
                    info!(task_id, browser_opens, "Browser window opened ({}/{})", browser_opens, MAX_BROWSER_OPENS);
                    // Add delay between browser opens to let pages load
                    if browser_opens > 1 {
                        tokio::time::sleep(std::time::Duration::from_millis(BROWSER_OPEN_DELAY_MS)).await;
                    }
                }

                emit(app_handle, "agent:step_started", task_id, step_num, expl);

                let is_gui = script.to_lowercase().contains("start-process");
                let timeout = if is_gui { 30 } else { settings.cli_timeout };

                let (success, output) = execute_with_retry(
                    &script, timeout, description, &gateway, settings, task_id,
                ).await;

                if is_gui { tokio::time::sleep(std::time::Duration::from_millis(1500)).await; }

                let sp = take_screenshot(screenshots_dir).await;
                let sp_str = sp.as_ref().map(|p| p.to_string_lossy().to_string());

                let action = AgentAction::RunCommand { command: script.clone(), shell: ShellType::PowerShell };
                let result = ExecutionResult {
                    method: ExecutionMethod::Terminal, success,
                    output: Some(output.clone()), screenshot_path: sp_str.clone(),
                    duration_ms: start.elapsed().as_millis() as u64,
                };
                save_step(db_path, task_id, step_num, &action, &sp.unwrap_or_default(), &result);
                emit(app_handle, "agent:step_completed", task_id, step_num, &format!("success={}", success));

                step_history.push(StepRecord {
                    step_number: step_num, action, result, screenshot_path: sp_str,
                });

                // Decide if we need a follow-up turn
                let remaining_steps = plan_json["remaining_steps"].as_u64().unwrap_or(0);
                if remaining_steps > 0 || (!success && turn < MAX_TURNS - 1) {
                    // Ask LLM what to do next
                    let remaining_msg = if remaining_steps > 0 {
                        format!("There are {} more steps planned.", remaining_steps)
                    } else if !success {
                        "The command failed. Try a different approach.".to_string()
                    } else {
                        "Decide if the task is complete or needs more work.".to_string()
                    };
                    let followup = FOLLOWUP_PROMPT
                        .replace("{COMMAND}", &script)
                        .replace("{SUCCESS}", if success { "true" } else { "false" })
                        .replace("{OUTPUT}", &output[..output.len().min(2000)])
                        .replace("{TASK}", description)
                        .replace("{REMAINING}", &remaining_msg);

                    conversation.push(("user".into(), followup.clone()));

                    if let Ok(next) = gateway.complete_as_agent(&followup, SYSTEM_PROMPT, settings).await {
                        current_response = next.content.trim().to_string();
                        conversation.push(("assistant".into(), current_response.clone()));
                        continue; // Next turn
                    }
                }

                accumulated_output = output;
                update_task_status(db_path, task_id, if success { "completed" } else { "failed" });
                break;
            }

            "command_then_screen" => {
                // Hybrid mode: run commands first, then switch to screen/vision
                let cmds = plan_json["commands"].as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                    .unwrap_or_default();
                let screen_task = plan_json["screen_task"].as_str().unwrap_or(description);
                let expl = plan_json["explanation"].as_str().unwrap_or("");

                // Phase 1: Execute commands
                if !cmds.is_empty() {
                    let script = cmds.join("; ");

                    // Browser spam guard
                    let opens = count_browser_opens(&script);
                    if opens > 0 {
                        browser_opens += opens;
                        if browser_opens > MAX_BROWSER_OPENS {
                            warn!(task_id, browser_opens, "Browser open limit reached in command_then_screen");
                            accumulated_output = format!("Stopped: opened {} browser windows (limit is {}). Use Invoke-WebRequest instead.", browser_opens, MAX_BROWSER_OPENS);
                            update_task_status(db_path, task_id, "failed");
                            break;
                        }
                        info!(task_id, browser_opens, "Browser window opened ({}/{})", browser_opens, MAX_BROWSER_OPENS);
                    }

                    emit(app_handle, "agent:step_started", task_id, 1, expl);
                    info!(task_id, script = %script, "command_then_screen: command phase");

                    let is_gui = script.to_lowercase().contains("start-process");
                    let timeout = if is_gui { 30 } else { settings.cli_timeout };
                    let (success, output) = execute_with_retry(&script, timeout, description, &gateway, settings, task_id).await;

                    if is_gui { tokio::time::sleep(std::time::Duration::from_secs(3)).await; }

                    let sp = take_screenshot(screenshots_dir).await;
                    let sp_str = sp.as_ref().map(|p| p.to_string_lossy().to_string());
                    let action = AgentAction::RunCommand { command: script.clone(), shell: ShellType::PowerShell };
                    let result = ExecutionResult {
                        method: ExecutionMethod::Terminal, success,
                        output: Some(output.clone()), screenshot_path: sp_str.clone(),
                        duration_ms: start.elapsed().as_millis() as u64,
                    };
                    save_step(db_path, task_id, 1, &action, &sp.unwrap_or_default(), &result);
                    step_history.push(StepRecord {
                        step_number: 1, action, result, screenshot_path: sp_str,
                    });

                    if !success {
                        accumulated_output = format!("Command phase failed: {}", output);
                        update_task_status(db_path, task_id, "failed");
                        break;
                    }
                }

                // Phase 2: Vision-guided screen interaction
                info!(task_id, screen_task, "command_then_screen: screen phase");
                let combined_task = format!("{}\n\nThe commands have already run. Now handle the visual part: {}", description, screen_task);

                // Minimize self so we don't capture our own window
                if let Some(win) = app_handle.get_webview_window("main") {
                    let _ = win.minimize();
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }

                let mut recent_actions: Vec<String> = Vec::new();

                for vs in 1..=15u32 {
                    if kill_switch.load(Ordering::Relaxed) { break; }

                    let screenshot = tokio::task::spawn_blocking({
                        let sd = screenshots_dir.to_path_buf();
                        move || {
                            let data = capture::capture_full_screen().map_err(|e| e.to_string())?;
                            let cap_w = data.width;
                            let cap_h = data.height;
                            let path = capture::save_screenshot(&data, &sd).map_err(|e| e.to_string())?;
                            let (b64, img_w, img_h) = capture::to_base64_jpeg_with_dims(&data, 80).map_err(|e| e.to_string())?;
                            Ok::<_, String>((path, b64, img_w, img_h, cap_w, cap_h))
                        }
                    }).await.map_err(|e| e.to_string())??;

                    let (sp, b64, img_w, img_h, cap_w, cap_h) = screenshot;
                    let step_num = 10 + vs;
                    emit(app_handle, "agent:step_started", task_id, step_num, "Screen interaction");

                    // Check for action repetition (before LLM call so warning fires on 2nd repeat)
                    let dedup_warning = recent_actions.len() >= 2
                        && recent_actions[recent_actions.len() - 1] == recent_actions[recent_actions.len() - 2];

                    let action = match vision::plan_next_action(
                        &b64, &combined_task, &step_history, settings, &gateway,
                        Some((img_w, img_h)), dedup_warning,
                    ).await {
                        Ok(a) => a,
                        Err(e) => {
                            warn!(task_id, error = %e, "Vision failed in command_then_screen");
                            accumulated_output = format!("Commands ran successfully but screen interaction failed: {}", e);
                            break;
                        }
                    };

                    // Track action for dedup BEFORE execution so warning fires earlier
                    recent_actions.push(format!("{:?}", action));

                    if let AgentAction::TaskComplete { ref summary } = action {
                        accumulated_output = summary.clone();
                        update_task_status(db_path, task_id, "completed");
                        step_history.push(StepRecord {
                            step_number: step_num, action,
                            result: ExecutionResult {
                                method: ExecutionMethod::Screen, success: true,
                                output: Some(accumulated_output.clone()),
                                screenshot_path: Some(sp.to_string_lossy().to_string()),
                                duration_ms: 0,
                            },
                            screenshot_path: Some(sp.to_string_lossy().to_string()),
                        });
                        break;
                    }

                    // Scale coords using physical capture dimensions (DPI-safe)
                    let scaled_action = scale_action_coords(action, img_w, img_h, cap_w, cap_h);
                    info!(task_id, step = step_num, action = ?scaled_action, "Vision action (scaled)");

                    let result = match executor::execute(&scaled_action, settings.cli_timeout, kill_switch).await {
                        Ok(r) => r,
                        Err(e) => ExecutionResult {
                            method: ExecutionMethod::Screen, success: false,
                            output: Some(e), screenshot_path: None, duration_ms: 0,
                        },
                    };

                    save_step(db_path, task_id, step_num, &scaled_action, &sp, &result);
                    step_history.push(StepRecord {
                        step_number: step_num, action: scaled_action, result,
                        screenshot_path: Some(sp.to_string_lossy().to_string()),
                    });

                    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                }

                // Restore window
                if let Some(win) = app_handle.get_webview_window("main") {
                    if let Err(e) = win.unminimize() {
                        warn!("Failed to restore window: {}", e);
                    }
                }

                if accumulated_output.is_empty() {
                    accumulated_output = "Task completed (command + screen interaction)".to_string();
                    update_task_status(db_path, task_id, "completed");
                }
            }

            "screen" => {
                let reason = plan_json["reason"].as_str().unwrap_or("UI task");
                info!(task_id, reason, "Vision mode");

                // Minimize self so we don't capture our own window
                if let Some(win) = app_handle.get_webview_window("main") {
                    let _ = win.minimize();
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }

                let mut recent_actions: Vec<String> = Vec::new();

                for vs in 1..=15u32 {
                    if kill_switch.load(Ordering::Relaxed) { break; }

                    let screenshot = tokio::task::spawn_blocking({
                        let sd = screenshots_dir.to_path_buf();
                        move || {
                            let data = capture::capture_full_screen().map_err(|e| e.to_string())?;
                            let cap_w = data.width;
                            let cap_h = data.height;
                            let path = capture::save_screenshot(&data, &sd).map_err(|e| e.to_string())?;
                            let (b64, img_w, img_h) = capture::to_base64_jpeg_with_dims(&data, 80).map_err(|e| e.to_string())?;
                            Ok::<_, String>((path, b64, img_w, img_h, cap_w, cap_h))
                        }
                    }).await.map_err(|e| e.to_string())??;

                    let (sp, b64, img_w, img_h, cap_w, cap_h) = screenshot;

                    // Check for action repetition (before LLM call so warning fires on 2nd repeat)
                    let dedup_warning = recent_actions.len() >= 2
                        && recent_actions[recent_actions.len() - 1] == recent_actions[recent_actions.len() - 2];

                    let action = match vision::plan_next_action(
                        &b64, description, &step_history, settings, &gateway,
                        Some((img_w, img_h)), dedup_warning,
                    ).await {
                        Ok(a) => a,
                        Err(e) => { accumulated_output = format!("Vision error: {}", e); break; }
                    };

                    // Track action for dedup BEFORE execution so warning fires earlier
                    recent_actions.push(format!("{:?}", action));

                    if let AgentAction::TaskComplete { ref summary } = action {
                        accumulated_output = summary.clone();
                        update_task_status(db_path, task_id, "completed");
                        break;
                    }

                    // Scale coords using physical capture dimensions (DPI-safe)
                    let scaled_action = scale_action_coords(action, img_w, img_h, cap_w, cap_h);
                    info!(task_id, step = vs, action = ?scaled_action, "Vision action (scaled)");

                    let result = match executor::execute(&scaled_action, settings.cli_timeout, kill_switch).await {
                        Ok(r) => r,
                        Err(e) => ExecutionResult {
                            method: ExecutionMethod::Screen, success: false,
                            output: Some(e), screenshot_path: None, duration_ms: 0,
                        },
                    };

                    save_step(db_path, task_id, turn * 10 + vs, &scaled_action, &sp, &result);
                    step_history.push(StepRecord {
                        step_number: turn * 10 + vs, action: scaled_action, result,
                        screenshot_path: Some(sp.to_string_lossy().to_string()),
                    });

                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }

                // Restore window
                if let Some(win) = app_handle.get_webview_window("main") {
                    if let Err(e) = win.unminimize() {
                        warn!("Failed to restore window: {}", e);
                    }
                }

                if accumulated_output.is_empty() {
                    accumulated_output = "Screen task completed".to_string();
                }
                update_task_status(db_path, task_id, "completed");
                break;
            }

            "done" => {
                accumulated_output = plan_json["output"].as_str()
                    .or(plan_json["summary"].as_str())
                    .unwrap_or("Task completed").to_string();
                update_task_status(db_path, task_id, "completed");
                break;
            }

            "chat" => {
                accumulated_output = plan_json["response"].as_str()
                    .unwrap_or("").to_string();
                update_task_status(db_path, task_id, "completed");
                break;
            }

            "need_info" => {
                accumulated_output = plan_json["question"].as_str()
                    .unwrap_or("I need more information").to_string();
                update_task_status(db_path, task_id, "completed");
                break;
            }

            _ => {
                accumulated_output = current_response.clone();
                update_task_status(db_path, task_id, "completed");
                break;
            }
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
    script: &str, timeout: u64, task: &str,
    gateway: &Gateway, settings: &Settings, task_id: &str,
) -> (bool, String) {
    let mut current = script.to_string();

    for attempt in 0..=MAX_RETRIES {
        match hands::cli::run_powershell(&current, timeout).await {
            Ok(out) => {
                if out.exit_code == 0 {
                    let text = if !out.stdout.trim().is_empty() { out.stdout }
                        else { "Command executed successfully.".into() };
                    return (true, text);
                }

                let err = if !out.stderr.trim().is_empty() { out.stderr.clone() }
                    else { format!("Exit code: {}", out.exit_code) };

                if attempt < MAX_RETRIES {
                    info!(task_id, attempt, "Auto-correcting failed command");
                    let prompt = format!("Original task: \"{}\"\n\n{}",
                        task, RETRY_PROMPT.replace("{ERROR}", &err).replace("{COMMAND}", &current));

                    if let Ok(fix) = gateway.complete_as_agent(&prompt, SYSTEM_PROMPT, settings).await {
                        if let Some(j) = extract_json(fix.content.trim()) {
                            if let Some(cmds) = j["commands"].as_array() {
                                let new: Vec<&str> = cmds.iter().filter_map(|v| v.as_str()).collect();
                                if !new.is_empty() { current = new.join("; "); continue; }
                            }
                        }
                    }
                }
                return (false, format!("Error: {}", err));
            }
            Err(e) => {
                if attempt >= MAX_RETRIES { return (false, format!("Execution error: {}", e)); }
            }
        }
    }
    (false, "Failed after retries".into())
}

// ═══════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════

/// Counts how many browser/URL opens a script triggers
fn count_browser_opens(script: &str) -> u32 {
    let lower = script.to_lowercase();
    let mut count = 0;
    // Start-Process with URLs
    for pattern in ["start-process 'http", "start-process \"http", "start-process http",
                     "start 'http", "start \"http", "start http",
                     "invoke-item 'http", "invoke-item \"http"] {
        count += lower.matches(pattern).count() as u32;
    }
    count
}

/// Decompose a complex task into subtasks using LLM
pub async fn decompose_task(
    description: &str,
    settings: &Settings,
) -> Result<Vec<String>, String> {
    let gateway = Gateway::new(settings);
    let prompt = format!(
        "Break this task into 2-5 concrete subtasks. Return ONLY a JSON array of strings, each being one subtask.\n\nTask: \"{}\"\n\nExample: [\"Research topic X\", \"Create comparison table\", \"Write summary\"]\n\nReturn ONLY the JSON array, nothing else.",
        description
    );

    let response = gateway.complete(&prompt, settings).await?;
    let content = response.content.trim();

    // Parse JSON array
    if let Some(arr) = extract_json(content).and_then(|v| v.as_array().cloned()) {
        let subtasks: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
        if !subtasks.is_empty() {
            return Ok(subtasks);
        }
    }

    // Fallback: try to parse as array directly
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(content) {
        if !arr.is_empty() {
            return Ok(arr);
        }
    }

    // Can't decompose — return single task
    Ok(vec![description.to_string()])
}

pub fn extract_json(text: &str) -> Option<serde_json::Value> {
    serde_json::from_str(text).ok().or_else(|| {
        let s = text.find('{')?;
        let e = text.rfind('}')?;
        serde_json::from_str(&text[s..=e]).ok()
    })
}

fn emit(app: &tauri::AppHandle, event: &str, task_id: &str, step: u32, desc: &str) {
    let _ = app.emit(event, serde_json::json!({
        "task_id": task_id, "step_number": step, "description": desc,
    }));
}

fn fail(db_path: &Path, task_id: &str, error: &str) {
    save_task_output(db_path, task_id, &format!("Error: {}", error));
    update_task_status(db_path, task_id, "failed");
}

async fn take_screenshot(dir: &Path) -> Option<std::path::PathBuf> {
    let d = dir.to_path_buf();
    tokio::task::spawn_blocking(move || {
        capture::capture_full_screen().ok().and_then(|data| capture::save_screenshot(&data, &d).ok())
    }).await.ok().flatten()
}

fn update_task_status(p: &Path, id: &str, s: &str) {
    if let Ok(db) = Database::new(p) { let _ = db.update_task_status(id, s); }
}

fn save_task_output(p: &Path, id: &str, o: &str) {
    if let Ok(db) = Database::new(p) { let _ = db.update_task_output(id, o); }
}

fn save_step(p: &Path, id: &str, n: u32, a: &AgentAction, sp: &Path, r: &ExecutionResult) {
    if let Ok(db) = Database::new(p) {
        let at = match a {
            AgentAction::Click{..}=>"click", AgentAction::DoubleClick{..}=>"double_click",
            AgentAction::RightClick{..}=>"right_click", AgentAction::Type{..}=>"type",
            AgentAction::KeyCombo{..}=>"key_combo", AgentAction::Scroll{..}=>"scroll",
            AgentAction::RunCommand{..}=>"run_command", AgentAction::Wait{..}=>"wait",
            AgentAction::Screenshot=>"screenshot", AgentAction::TaskComplete{..}=>"task_complete",
        };
        let em = match r.method { ExecutionMethod::Api=>"api", ExecutionMethod::Terminal=>"terminal", ExecutionMethod::Screen=>"screen" };
        let _ = db.insert_task_step(id, n, at, &serde_json::to_string(a).unwrap_or_default(),
            &sp.to_string_lossy(), em, r.success, r.duration_ms);
    }
}
