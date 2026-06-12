# Kaubo Benchmark — 编译+执行分离
param([int]$Runs = 4)

$kauboExe = (Resolve-Path "target\release\kaubo.exe").Path

# === Kaubo ===
Write-Host "=== Kaubo ===" -ForegroundColor Cyan
$times = @()
for ($i = 1; $i -le $Runs; $i++) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    & $kauboExe bench\package.json 2>$null
    $sw.Stop()
    $times += $sw.Elapsed.TotalMilliseconds
    $label = if ($i -eq 1) { " (incl compile)" } else { "" }
    Write-Host "  Run $i : $([math]::Round($sw.Elapsed.TotalMilliseconds))ms$label" -ForegroundColor Cyan
}
$compileTime = $times[0] - $times[1]
$avgExec = ($times[1..($times.Count-1)] | Measure-Object -Average).Average
Write-Host "  Compile: $([math]::Round($compileTime))ms" -ForegroundColor DarkGray
Write-Host "  Execute: $([math]::Round($avgExec))ms (avg)" -ForegroundColor Cyan

# === Python ===
Write-Host "`n=== Python ===" -ForegroundColor Yellow
$py_times = @()
for ($i = 1; $i -le $Runs; $i++) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    python bench\bench.py
    $sw.Stop()
    $py_times += $sw.Elapsed.TotalMilliseconds
}
$avgPy = ($py_times | Measure-Object -Average).Average
Write-Host "  Execute: $([math]::Round($avgPy))ms (avg)" -ForegroundColor Yellow

# === Rust ===
Write-Host "`n=== Rust ===" -ForegroundColor Green
if (-not (Test-Path bench\bench_rs.exe)) { rustc -O bench\bench.rs -o bench\bench_rs.exe 2>$null }
$rs_times = @()
for ($i = 1; $i -le $Runs; $i++) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    & bench\bench_rs.exe
    $sw.Stop()
    $rs_times += $sw.Elapsed.TotalMilliseconds
}
$avgRs = ($rs_times | Measure-Object -Average).Average
Write-Host "  Execute: $([math]::Round($avgRs))ms (avg)" -ForegroundColor Green

# === Summary ===
Write-Host "`n=== Summary ===" -ForegroundColor White
Write-Host ("  Kaubo  compile: {0,6}ms" -f [math]::Round($compileTime))
Write-Host ("  Kaubo  execute: {0,6}ms" -f [math]::Round($avgExec))
Write-Host ("  Python execute: {0,6}ms" -f [math]::Round($avgPy))
Write-Host ("  Rust   execute: {0,6}ms" -f [math]::Round($avgRs))
Write-Host ""
Write-Host ("  Kaubo vs Python (exec): {0:F1}x" -f ($avgExec / [Math]::Max(1, $avgPy)))
Write-Host ("  Kaubo vs Rust   (exec): {0:F0}x" -f ($avgExec / [Math]::Max(1, $avgRs)))
