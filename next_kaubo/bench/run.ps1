# Kaubo Benchmark Suite — 一键运行 Kaubo / Python / Rust
param([switch]$Quick)

$sw_all = [System.Diagnostics.Stopwatch]::StartNew()

Write-Host "=== Kaubo ===" -ForegroundColor Cyan
$sw = [System.Diagnostics.Stopwatch]::StartNew()
& target\release\kaubo.exe bench\package.json 2>$null
$sw.Stop()
Write-Host "Kaubo: $([math]::Round($sw.Elapsed.TotalMilliseconds))ms" -ForegroundColor Cyan

Write-Host "`n=== CPython $((python -c 'import sys;print(sys.version.split()[0])') 2>$null) ===" -ForegroundColor Yellow
$sw = [System.Diagnostics.Stopwatch]::StartNew()
python bench\bench.py
$sw.Stop()
Write-Host "Python: $([math]::Round($sw.Elapsed.TotalMilliseconds))ms" -ForegroundColor Yellow

if (-not $Quick) {
    if (-not (Test-Path bench\bench_rs.exe)) {
        Write-Host "`nCompiling Rust..." -ForegroundColor DarkGray
        rustc -O bench\bench.rs -o bench\bench_rs.exe 2>&1 | Out-Null
    }
    Write-Host "`n=== Rust ===" -ForegroundColor Green
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    & bench\bench_rs.exe
    $sw.Stop()
    Write-Host "Rust: $([math]::Round($sw.Elapsed.TotalMilliseconds))ms" -ForegroundColor Green
}

$sw_all.Stop()
Write-Host "`nTotal: $([math]::Round($sw_all.Elapsed.TotalMilliseconds))ms" -ForegroundColor White
