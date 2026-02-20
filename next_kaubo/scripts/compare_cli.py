#!/usr/bin/env python3
"""
对比测试：验证新旧 CLI 行为一致性
"""

import subprocess
import sys
import os
from pathlib import Path

# 测试项目列表
TEST_PROJECTS = [
    "examples/test_simple",
    "examples/fib", 
    "examples/calc",
    "examples/hello",
]

def run_cmd(cmd, cwd=None):
    """运行命令并返回结果"""
    try:
        result = subprocess.run(
            cmd,
            shell=True,
            capture_output=True,
            text=True,
            cwd=cwd,
            timeout=30
        )
        return result.returncode, result.stdout, result.stderr
    except subprocess.TimeoutExpired:
        return -1, "", "Timeout"

def test_old_cli(project):
    """测试旧 CLI"""
    cmd = f"cargo run -p kaubo-cli -- {project}/package.json --verbose"
    return run_cmd(cmd)

def test_new_cli(project):
    """测试新 CLI"""
    cmd = f"cargo run -p kaubo-cli-orchestrator -- {project}/package.json --verbose"
    return run_cmd(cmd)

def main():
    print("=" * 60)
    print("CLI 对比测试")
    print("=" * 60)
    
    all_passed = True
    
    for project in TEST_PROJECTS:
        print(f"\n测试项目: {project}")
        print("-" * 40)
        
        # 检查项目是否存在
        if not Path(f"{project}/package.json").exists():
            print(f"  ⚠️  跳过 (项目不存在)")
            continue
        
        # 测试旧 CLI
        print("  旧 CLI (kaubo)...")
        old_code, old_out, old_err = test_old_cli(project)
        old_success = old_code == 0
        print(f"    {'✅ 成功' if old_success else '❌ 失败'} (exit code: {old_code})")
        
        # 测试新 CLI
        print("  新 CLI (kaubo2)...")
        new_code, new_out, new_err = test_new_cli(project)
        new_success = new_code == 0
        print(f"    {'✅ 成功' if new_success else '❌ 失败'} (exit code: {new_code})")
        
        # 对比结果
        if old_success == new_success:
            print(f"  ✅ 行为一致")
        else:
            print(f"  ❌ 行为不一致!")
            print(f"    旧 CLI: {'成功' if old_success else '失败'}")
            print(f"    新 CLI: {'成功' if new_success else '失败'}")
            all_passed = False
            
            # 显示错误信息
            if not new_success:
                print(f"\n  新 CLI 错误输出:")
                for line in new_err.split('\n')[-5:]:
                    if line.strip():
                        print(f"    {line}")
    
    print("\n" + "=" * 60)
    if all_passed:
        print("✅ 所有测试通过，新旧 CLI 行为一致")
    else:
        print("❌ 部分测试失败，需要修复")
    print("=" * 60)
    
    return 0 if all_passed else 1

if __name__ == "__main__":
    sys.exit(main())
