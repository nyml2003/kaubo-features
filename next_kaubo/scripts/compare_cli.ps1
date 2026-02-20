# 对比测试：验证新旧 CLI 行为一致性

$projects = @(
    "examples/test_simple",
    "examples/fib",
    "examples/calc"
)

Write-Host "============================================================"
Write-Host "CLI 对比测试"
Write-Host "============================================================"

$allPassed = $true

foreach ($project in $projects) {
    Write-Host ""
    Write-Host "测试项目: $project"
    Write-Host "----------------------------------------"
    
    if (-not (Test-Path "$project/package.json")) {
        Write-Host "  跳过 (项目不存在)"
        continue
    }
    
    # 测试旧 CLI
    Write-Host "  旧 CLI (kaubo)..."
    $oldOutput = cargo run -p kaubo-cli -- "$project/package.json" 2>&1
    $oldSuccess = $LASTEXITCODE -eq 0
    Write-Host "    $(if ($oldSuccess) { '成功' } else { '失败' }) (exit code: $LASTEXITCODE)"
    
    # 测试新 CLI
    Write-Host "  新 CLI (kaubo2)..."
    $newOutput = cargo run -p kaubo-cli-orchestrator -- "$project/package.json" 2>&1
    $newSuccess = $LASTEXITCODE -eq 0
    Write-Host "    $(if ($newSuccess) { '成功' } else { '失败' }) (exit code: $LASTEXITCODE)"
    
    # 对比
    if ($oldSuccess -eq $newSuccess) {
        Write-Host "  行为一致"
    } else {
        Write-Host "  行为不一致!"
        $allPassed = $false
    }
}

Write-Host ""
Write-Host "============================================================"
if ($allPassed) {
    Write-Host "所有测试通过，新旧 CLI 行为一致"
} else {
    Write-Host "部分测试失败，需要修复"
}
Write-Host "============================================================"
