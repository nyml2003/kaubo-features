#!/usr/bin/env python3
"""
覆盖率报告生成脚本
使用标准库，兼容 Python 3.6+
"""

import subprocess
import sys
import os
import argparse
import webbrowser


def run_command(cmd, description):
    """运行命令并打印结果"""
    print(f"\n{'='*50}")
    print(f"正在执行: {description}")
    print(f"命令: {' '.join(cmd)}")
    print('='*50)
    
    result = subprocess.run(cmd, capture_output=True, text=True)
    
    if result.stdout:
        print(result.stdout)
    if result.stderr:
        print(result.stderr, file=sys.stderr)
    
    return result.returncode == 0


def parse_coverage_output(text):
    """从 tarpaulin 输出中解析覆盖率"""
    for line in text.split('\n'):
        if '% coverage' in line and 'lines covered' in line:
            return line.strip()
    return None


def main():
    parser = argparse.ArgumentParser(description='生成测试覆盖率报告')
    parser.add_argument('--html', action='store_true', help='生成 HTML 报告')
    parser.add_argument('--open', action='store_true', help='打开 HTML 报告')
    args = parser.parse_args()
    
    # 检查 tarpaulin 是否安装
    check = subprocess.run(['cargo', 'tarpaulin', '--version'], 
                          capture_output=True, text=True)
    if check.returncode != 0:
        print("错误: cargo-tarpaulin 未安装")
        print("请运行: cargo install cargo-tarpaulin")
        sys.exit(1)
    
    # 构建 tarpaulin 命令
    cmd = [
        'cargo', 'tarpaulin',
        '--include-tests',
        '--all-targets',
        '--output-dir', 'target/tarpaulin'
    ]
    
    if args.html or args.open:
        cmd.extend(['--out', 'Html', '--out', 'Xml'])
    # 默认输出到终端，不需要额外参数
    
    # 运行覆盖率测试
    print("开始生成覆盖率报告...")
    success = run_command(cmd, "覆盖率测试")
    
    if not success:
        print("\n覆盖率测试失败!")
        sys.exit(1)
    
    # 打印摘要
    print("\n" + "="*50)
    print("覆盖率测试完成!")
    print("="*50)
    
    if args.html or args.open:
        html_path = os.path.join('target', 'tarpaulin', 'tarpaulin-report.html')
        xml_path = os.path.join('target', 'tarpaulin', 'cobertura.xml')
        
        print(f"\n报告文件:")
        print(f"  HTML: {os.path.abspath(html_path)}")
        print(f"  XML:  {os.path.abspath(xml_path)}")
        
        if args.open and os.path.exists(html_path):
            print(f"\n正在打开报告...")
            webbrowser.open(f'file://{os.path.abspath(html_path)}')
    
    print()


if __name__ == '__main__':
    main()
