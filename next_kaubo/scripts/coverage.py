#!/usr/bin/env python3
"""
覆盖率报告生成脚本 (使用 cargo-llvm-cov + nightly)
支持行覆盖率和分支覆盖率
使用标准库，兼容 Python 3.6+
"""

import subprocess
import sys
import os
import argparse
import webbrowser


def check_tool(name, command):
    """检查工具是否安装"""
    result = subprocess.run(command, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"错误: {name} 未安装")
        print(f"请运行: cargo install {name}")
        return False
    print(f"{name} 已安装: {result.stdout.strip()}")
    return True


def check_nightly():
    """检查 nightly 工具链是否安装"""
    result = subprocess.run(['rustup', 'show'], capture_output=True, text=True)
    if 'nightly' in result.stdout:
        print("nightly 工具链已安装")
        return True
    print("错误: nightly 工具链未安装")
    print("请运行: rustup install nightly")
    return False


def run_coverage(html=False, open_browser=False):
    """运行覆盖率测试 (使用 nightly + 分支覆盖率)"""
    cmd = [
        'cargo', '+nightly', 'llvm-cov',
        '--branch',           # 启用分支覆盖率 (需要 nightly)
        '--all-features',     # 测试所有特性
    ]
    
    output_dir = 'target/llvm-cov'
    html_file = os.path.join(output_dir, 'index.html')
    
    if html or open_browser:
        cmd.extend(['--html', '--output-dir', output_dir])
    
    print("\n" + "="*50)
    print("正在运行覆盖率测试")
    print("工具: cargo-llvm-cov + nightly (支持分支覆盖率)")
    print("="*50)
    print(f"命令: {' '.join(cmd)}")
    print()
    
    result = subprocess.run(cmd)
    
    if result.returncode != 0:
        print("\n覆盖率测试失败!")
        return False
    
    print("\n" + "="*50)
    print("覆盖率测试完成!")
    print("="*50)
    
    if html or open_browser:
        print(f"\n报告位置: {os.path.abspath(output_dir)}")
        print(f"HTML 文件: {os.path.abspath(html_file)}")
        
        if open_browser and os.path.exists(html_file):
            print(f"\n正在打开浏览器...")
            webbrowser.open(f'file://{os.path.abspath(html_file)}')
    
    print()
    return True


def main():
    parser = argparse.ArgumentParser(
        description='生成测试覆盖率报告 (cargo-llvm-cov + nightly, 支持分支覆盖率)'
    )
    parser.add_argument('--html', action='store_true', help='生成 HTML 报告')
    parser.add_argument('--open', action='store_true', help='生成并打开 HTML 报告')
    args = parser.parse_args()
    
    # 检查依赖
    if not check_tool('cargo-llvm-cov', ['cargo', 'llvm-cov', '--version']):
        sys.exit(1)
    
    if not check_nightly():
        sys.exit(1)
    
    # 运行覆盖率测试
    success = run_coverage(html=args.html, open_browser=args.open)
    
    if not success:
        sys.exit(1)


if __name__ == '__main__':
    main()
