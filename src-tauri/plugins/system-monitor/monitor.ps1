$cpu = (Get-CimInstance Win32_Processor).LoadPercentage
$ram = Get-CimInstance Win32_OperatingSystem
$ramUsed = [math]::Round(($ram.TotalVisibleMemorySize - $ram.FreePhysicalMemory) / 1MB, 1)
$ramTotal = [math]::Round($ram.TotalVisibleMemorySize / 1MB, 1)
$disk = Get-CimInstance Win32_LogicalDisk -Filter "DeviceID='C:'"
$diskUsed = [math]::Round(($disk.Size - $disk.FreeSpace) / 1GB, 1)
$diskTotal = [math]::Round($disk.Size / 1GB, 1)
Write-Output "CPU: ${cpu}% | RAM: ${ramUsed}/${ramTotal} GB | Disk: ${diskUsed}/${diskTotal} GB"
